//! Drafts, signatures, and attachment uploads for the composer.

use axum::extract::Multipart;
use serde::Serialize;
use serde_json::Value;
use sqlx::PgPool;
use sqlx::types::Json;
use uuid::Uuid;

use crate::api::AppError;
use crate::auth::{CurrentUser, pool_of};
use crate::compose::{
    MAX_FILE_BYTES, MAX_FILES, MAX_TOTAL_BYTES, content_type, inline_preview, outbound_filename,
    sanitize_html,
};
use crate::state::AppState;

use super::access::require_reader;

#[derive(Serialize)]
pub struct DraftView {
    pub id: Uuid,
    pub mailbox_id: Uuid,
    pub compose_kind: String,
    pub in_reply_to_message_id: Option<Uuid>,
    pub to: Vec<String>,
    pub cc: Vec<String>,
    pub bcc: Vec<String>,
    pub subject: String,
    pub body_html: String,
    pub body_text: String,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub attachments: Vec<FileView>,
}

#[derive(Serialize, sqlx::FromRow)]
pub struct FileView {
    pub id: Uuid,
    pub filename: String,
    pub content_type: String,
    pub size_bytes: i64,
    pub scan_status: String,
}

#[derive(Serialize)]
pub struct SignatureView {
    pub id: Uuid,
    pub mailbox_id: Option<Uuid>,
    pub name: String,
    pub body_html: String,
    pub is_default: bool,
}

#[derive(Serialize, sqlx::FromRow)]
pub struct Suggestion {
    pub address: String,
    pub name: Option<String>,
}

pub struct DraftInput {
    pub kind: String,
    pub in_reply_to: Option<Uuid>,
    pub to: Vec<String>,
    pub cc: Vec<String>,
    pub bcc: Vec<String>,
    pub subject: String,
    pub html: String,
}

pub struct Download {
    pub filename: String,
    pub content_type: String,
    pub inline: bool,
    pub bytes: Vec<u8>,
}

pub async fn create_draft(
    state: &AppState,
    user: &CurrentUser,
    mailbox_id: Uuid,
    input: DraftInput,
) -> Result<DraftView, AppError> {
    let pool = pool_of(state)?;
    require_reader(pool, mailbox_id, user).await?;
    let saved = clean_draft(input)?;
    let id = sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO drafts (
            mailbox_id, author_user_id, in_reply_to_message_id, compose_kind,
            to_recipients, cc_recipients, bcc_recipients, subject, body_html, body_text
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
        RETURNING id
        "#,
    )
    .bind(mailbox_id)
    .bind(user.id)
    .bind(saved.in_reply_to)
    .bind(&saved.kind)
    .bind(Json(saved.to.clone()))
    .bind(Json(saved.cc.clone()))
    .bind(Json(saved.bcc.clone()))
    .bind(&saved.subject)
    .bind(&saved.html)
    .bind(&saved.text)
    .fetch_one(pool)
    .await?;
    load_draft(pool, user.id, id).await
}

pub async fn update_draft(
    state: &AppState,
    user: &CurrentUser,
    draft_id: Uuid,
    input: DraftInput,
) -> Result<DraftView, AppError> {
    let pool = pool_of(state)?;
    let saved = clean_draft(input)?;
    let updated = sqlx::query(
        r#"
        UPDATE drafts
        SET compose_kind = $3,
            in_reply_to_message_id = $4,
            to_recipients = $5,
            cc_recipients = $6,
            bcc_recipients = $7,
            subject = $8,
            body_html = $9,
            body_text = $10
        WHERE id = $1 AND author_user_id = $2
        "#,
    )
    .bind(draft_id)
    .bind(user.id)
    .bind(&saved.kind)
    .bind(saved.in_reply_to)
    .bind(Json(saved.to.clone()))
    .bind(Json(saved.cc.clone()))
    .bind(Json(saved.bcc.clone()))
    .bind(&saved.subject)
    .bind(&saved.html)
    .bind(&saved.text)
    .execute(pool)
    .await?;
    if updated.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    load_draft(pool, user.id, draft_id).await
}

pub async fn read_draft(
    state: &AppState,
    user: &CurrentUser,
    draft_id: Uuid,
) -> Result<DraftView, AppError> {
    let pool = pool_of(state)?;
    load_draft(pool, user.id, draft_id).await
}

pub async fn delete_draft(
    state: &AppState,
    user: &CurrentUser,
    draft_id: Uuid,
) -> Result<(), AppError> {
    let pool = pool_of(state)?;
    let keys = sqlx::query_scalar::<_, String>(
        r#"
        SELECT a.storage_key
        FROM attachments a
        JOIN drafts d ON d.id = a.draft_id
        WHERE d.id = $1 AND d.author_user_id = $2
        "#,
    )
    .bind(draft_id)
    .bind(user.id)
    .fetch_all(pool)
    .await?;
    let removed = sqlx::query("DELETE FROM drafts WHERE id = $1 AND author_user_id = $2")
        .bind(draft_id)
        .bind(user.id)
        .execute(pool)
        .await?;
    if removed.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    for key in keys {
        state.storage.delete(&key).await;
    }
    Ok(())
}

pub async fn upload_file(
    state: &AppState,
    user: &CurrentUser,
    draft_id: Uuid,
    mut multipart: Multipart,
) -> Result<FileView, AppError> {
    let pool = pool_of(state)?;
    let mailbox_id = sqlx::query_scalar::<_, Uuid>(
        "SELECT mailbox_id FROM drafts WHERE id = $1 AND author_user_id = $2",
    )
    .bind(draft_id)
    .bind(user.id)
    .fetch_optional(pool)
    .await?
    .ok_or(AppError::NotFound)?;
    require_reader(pool, mailbox_id, user).await?;
    let mut filename = None;
    let mut mime = String::from("application/octet-stream");
    let mut bytes = None;
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|_| AppError::Validation("The upload could not be read.".into()))?
    {
        if field.name() != Some("file") {
            continue;
        }
        filename = Some(outbound_filename(field.file_name().unwrap_or("attachment"))?);
        mime = content_type(field.content_type().unwrap_or("application/octet-stream"));
        let data = field
            .bytes()
            .await
            .map_err(|_| AppError::Validation("The upload could not be read.".into()))?;
        if data.len() > MAX_FILE_BYTES {
            return Err(AppError::Validation(
                "Each attachment can be at most 10 MB.".into(),
            ));
        }
        if data.is_empty() {
            return Err(AppError::Validation("The file is empty.".into()));
        }
        bytes = Some(data);
        break;
    }
    let (filename, bytes) = filename
        .zip(bytes)
        .ok_or_else(|| AppError::Validation("Choose a file to attach.".into()))?;
    let usage = sqlx::query_as::<_, (i64, i64)>(
        r#"
        SELECT count(*)::bigint, COALESCE(sum(size_bytes), 0)::bigint
        FROM attachments
        WHERE draft_id = $1
        "#,
    )
    .bind(draft_id)
    .fetch_one(pool)
    .await?;
    if usage.0 >= MAX_FILES as i64 {
        return Err(AppError::Validation(
            "A message can have at most 10 attachments.".into(),
        ));
    }
    if usage.1 + bytes.len() as i64 > MAX_TOTAL_BYTES {
        return Err(AppError::Validation(
            "Attachments can be at most 25 MB together.".into(),
        ));
    }
    crate::limit::charge(pool, &format!("upload:{}", user.id), 3600, 60).await?;
    let scan_status = crate::scan::status_for(state.config.scanner.as_deref(), &bytes).await;
    let id = Uuid::new_v4();
    let key = format!("attachments/{id}");
    state.storage.put(&key, &bytes).await?;
    let inserted = sqlx::query_as::<_, FileView>(
        r#"
        INSERT INTO attachments (
            id, draft_id, filename, content_type, size_bytes, storage_key, scan_status
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        RETURNING id, filename, content_type, size_bytes, scan_status
        "#,
    )
    .bind(id)
    .bind(draft_id)
    .bind(&filename)
    .bind(&mime)
    .bind(bytes.len() as i64)
    .bind(&key)
    .bind(scan_status)
    .fetch_one(pool)
    .await;
    match inserted {
        Ok(file) => Ok(file),
        Err(error) => {
            state.storage.delete(&key).await;
            Err(error.into())
        }
    }
}

pub async fn remove_file(
    state: &AppState,
    user: &CurrentUser,
    draft_id: Uuid,
    file_id: Uuid,
) -> Result<(), AppError> {
    let pool = pool_of(state)?;
    let key = sqlx::query_scalar::<_, String>(
        r#"
        SELECT a.storage_key
        FROM attachments a
        JOIN drafts d ON d.id = a.draft_id
        WHERE a.id = $1 AND d.id = $2 AND d.author_user_id = $3
        "#,
    )
    .bind(file_id)
    .bind(draft_id)
    .bind(user.id)
    .fetch_optional(pool)
    .await?
    .ok_or(AppError::NotFound)?;
    sqlx::query("DELETE FROM attachments WHERE id = $1")
        .bind(file_id)
        .execute(pool)
        .await?;
    state.storage.delete(&key).await;
    Ok(())
}

pub async fn download(
    state: &AppState,
    user: &CurrentUser,
    file_id: Uuid,
) -> Result<Download, AppError> {
    let pool = pool_of(state)?;
    let row = sqlx::query_as::<_, OwnedFile>(
        r#"
        SELECT filename, content_type, storage_key, message_id, draft_id, scan_status
        FROM attachments
        WHERE id = $1
        "#,
    )
    .bind(file_id)
    .fetch_optional(pool)
    .await?
    .ok_or(AppError::NotFound)?;
    if let Some(message_id) = row.message_id {
        let mailbox_id = super::access::message_mailbox(pool, message_id).await?;
        require_reader(pool, mailbox_id, user)
            .await
            .map_err(|error| match error {
                AppError::Forbidden(_) => AppError::NotFound,
                other => other,
            })?;
    } else if let Some(draft_id) = row.draft_id {
        let author = sqlx::query_scalar::<_, Uuid>(
            "SELECT author_user_id FROM drafts WHERE id = $1",
        )
        .bind(draft_id)
        .fetch_optional(pool)
        .await?;
        if author != Some(user.id) {
            return Err(AppError::NotFound);
        }
    } else {
        return Err(AppError::NotFound);
    }
    if matches!(row.scan_status.as_str(), "rejected" | "failed") {
        return Err(AppError::Validation(
            "This attachment is not available.".into(),
        ));
    }
    let bytes = state.storage.get(&row.storage_key).await?;
    let inline = inline_preview(&row.content_type);
    Ok(Download {
        filename: row.filename,
        content_type: row.content_type,
        inline,
        bytes,
    })
}

pub async fn list_signatures(
    state: &AppState,
    user: &CurrentUser,
    mailbox_id: Uuid,
) -> Result<Vec<SignatureView>, AppError> {
    let pool = pool_of(state)?;
    require_reader(pool, mailbox_id, user).await?;
    let rows = sqlx::query_as::<_, SignatureRow>(
        r#"
        SELECT id, mailbox_id, name, body_html, is_default
        FROM email_signatures
        WHERE user_id = $1 AND (mailbox_id = $2 OR mailbox_id IS NULL)
        ORDER BY is_default DESC, name
        "#,
    )
    .bind(user.id)
    .bind(mailbox_id)
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().map(signature_view).collect())
}

pub async fn create_signature(
    state: &AppState,
    user: &CurrentUser,
    mailbox_id: Option<Uuid>,
    name: String,
    html: String,
    default: bool,
) -> Result<SignatureView, AppError> {
    let pool = pool_of(state)?;
    if let Some(mailbox_id) = mailbox_id {
        require_reader(pool, mailbox_id, user).await?;
    }
    let name = clean_name(&name)?;
    let html = sanitize_html(&html);
    let mut tx = pool.begin().await?;
    if default {
        clear_default(&mut tx, user.id, mailbox_id).await?;
    }
    let row = sqlx::query_as::<_, SignatureRow>(
        r#"
        INSERT INTO email_signatures (user_id, mailbox_id, name, body_html, is_default)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING id, mailbox_id, name, body_html, is_default
        "#,
    )
    .bind(user.id)
    .bind(mailbox_id)
    .bind(name)
    .bind(html)
    .bind(default)
    .fetch_one(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(signature_view(row))
}

pub async fn update_signature(
    state: &AppState,
    user: &CurrentUser,
    signature_id: Uuid,
    name: String,
    html: String,
    default: bool,
) -> Result<SignatureView, AppError> {
    let pool = pool_of(state)?;
    let name = clean_name(&name)?;
    let html = sanitize_html(&html);
    let mut tx = pool.begin().await?;
    let mailbox_id = sqlx::query_scalar::<_, Option<Uuid>>(
        "SELECT mailbox_id FROM email_signatures WHERE id = $1 AND user_id = $2",
    )
    .bind(signature_id)
    .bind(user.id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(AppError::NotFound)?;
    if default {
        clear_default(&mut tx, user.id, mailbox_id).await?;
    }
    let row = sqlx::query_as::<_, SignatureRow>(
        r#"
        UPDATE email_signatures
        SET name = $3, body_html = $4, is_default = $5
        WHERE id = $1 AND user_id = $2
        RETURNING id, mailbox_id, name, body_html, is_default
        "#,
    )
    .bind(signature_id)
    .bind(user.id)
    .bind(name)
    .bind(html)
    .bind(default)
    .fetch_one(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(signature_view(row))
}

pub async fn delete_signature(
    state: &AppState,
    user: &CurrentUser,
    signature_id: Uuid,
) -> Result<(), AppError> {
    let pool = pool_of(state)?;
    let removed =
        sqlx::query("DELETE FROM email_signatures WHERE id = $1 AND user_id = $2")
            .bind(signature_id)
            .bind(user.id)
            .execute(pool)
            .await?;
    if removed.rows_affected() == 0 {
        Err(AppError::NotFound)
    } else {
        Ok(())
    }
}

pub async fn suggest(
    state: &AppState,
    user: &CurrentUser,
    mailbox_id: Uuid,
    query: &str,
) -> Result<Vec<Suggestion>, AppError> {
    let pool = pool_of(state)?;
    require_reader(pool, mailbox_id, user).await?;
    let query = query.trim();
    if query.is_empty() || query.chars().count() > 80 || query.chars().any(char::is_control) {
        return Ok(Vec::new());
    }
    let pattern = format!("%{}%", query.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_"));
    let rows = sqlx::query_as::<_, Suggestion>(
        r#"
        SELECT address::text AS address, max(name) AS name
        FROM (
            SELECT r.address, r.name
            FROM recipients r
            JOIN messages m ON m.id = r.message_id
            WHERE m.mailbox_id = $1
              AND (
                    r.address::text ILIKE $2 ESCAPE '\'
                    OR COALESCE(r.name, '') ILIKE $2 ESCAPE '\'
              )
            UNION ALL
            SELECT c.email, c.full_name
            FROM contacts c
            WHERE c.owner_user_id = $3
              AND c.email IS NOT NULL
              AND (
                    c.email::text ILIKE $2 ESCAPE '\'
                    OR c.full_name ILIKE $2 ESCAPE '\'
              )
        ) people
        GROUP BY address
        ORDER BY address
        LIMIT 8
        "#,
    )
    .bind(mailbox_id)
    .bind(pattern)
    .bind(user.id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

struct SavedDraft {
    kind: String,
    in_reply_to: Option<Uuid>,
    to: Vec<String>,
    cc: Vec<String>,
    bcc: Vec<String>,
    subject: String,
    html: String,
    text: String,
}

fn clean_draft(input: DraftInput) -> Result<SavedDraft, AppError> {
    let kind = match input.kind.as_str() {
        "new" | "reply" | "reply-all" | "forward" => input.kind,
        _ => return Err(AppError::Validation("That compose kind is not known.".into())),
    };
    let subject = input.subject.trim().to_string();
    if subject.chars().count() > 998 || subject.chars().any(char::is_control) {
        return Err(AppError::Validation(
            "Enter a subject of at most 998 characters.".into(),
        ));
    }
    let html = sanitize_html(&input.html);
    let text = crate::compose::html_to_text(&html);
    if html.chars().count() > 200_000 {
        return Err(AppError::Validation("The message body is too long.".into()));
    }
    Ok(SavedDraft {
        kind,
        in_reply_to: input.in_reply_to,
        to: clean_addresses(input.to)?,
        cc: clean_addresses(input.cc)?,
        bcc: clean_addresses(input.bcc)?,
        subject,
        html,
        text,
    })
}

fn clean_addresses(values: Vec<String>) -> Result<Vec<String>, AppError> {
    if values.len() > 50 {
        return Err(AppError::Validation(
            "A message can have at most 50 recipients.".into(),
        ));
    }
    Ok(values
        .into_iter()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .collect())
}

fn clean_name(value: &str) -> Result<String, AppError> {
    let name = value.trim();
    if name.is_empty() || name.chars().count() > 80 || name.chars().any(char::is_control) {
        return Err(AppError::Validation(
            "Enter a signature name of 1 to 80 characters.".into(),
        ));
    }
    Ok(name.to_string())
}

async fn clear_default(
    tx: &mut sqlx::PgConnection,
    user_id: Uuid,
    mailbox_id: Option<Uuid>,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        UPDATE email_signatures
        SET is_default = false
        WHERE user_id = $1 AND mailbox_id IS NOT DISTINCT FROM $2 AND is_default
        "#,
    )
    .bind(user_id)
    .bind(mailbox_id)
    .execute(&mut *tx)
    .await?;
    Ok(())
}

async fn load_draft(pool: &PgPool, author_id: Uuid, draft_id: Uuid) -> Result<DraftView, AppError> {
    let row = sqlx::query_as::<_, DraftRow>(
        r#"
        SELECT id, mailbox_id, compose_kind, in_reply_to_message_id,
               to_recipients, cc_recipients, bcc_recipients,
               subject, body_html, body_text, updated_at
        FROM drafts
        WHERE id = $1 AND author_user_id = $2
        "#,
    )
    .bind(draft_id)
    .bind(author_id)
    .fetch_optional(pool)
    .await?
    .ok_or(AppError::NotFound)?;
    let attachments = sqlx::query_as::<_, FileView>(
        r#"
        SELECT id, filename, content_type, size_bytes, scan_status
        FROM attachments
        WHERE draft_id = $1
        ORDER BY created_at
        "#,
    )
    .bind(draft_id)
    .fetch_all(pool)
    .await?;
    Ok(DraftView {
        id: row.id,
        mailbox_id: row.mailbox_id,
        compose_kind: row.compose_kind,
        in_reply_to_message_id: row.in_reply_to_message_id,
        to: json_strings(row.to_recipients),
        cc: json_strings(row.cc_recipients),
        bcc: json_strings(row.bcc_recipients),
        subject: row.subject,
        body_html: row.body_html,
        body_text: row.body_text,
        updated_at: row.updated_at,
        attachments,
    })
}

fn json_strings(value: Value) -> Vec<String> {
    value
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

fn signature_view(row: SignatureRow) -> SignatureView {
    SignatureView {
        id: row.id,
        mailbox_id: row.mailbox_id,
        name: row.name,
        body_html: row.body_html,
        is_default: row.is_default,
    }
}

#[derive(sqlx::FromRow)]
struct DraftRow {
    id: Uuid,
    mailbox_id: Uuid,
    compose_kind: String,
    in_reply_to_message_id: Option<Uuid>,
    to_recipients: Value,
    cc_recipients: Value,
    bcc_recipients: Value,
    subject: String,
    body_html: String,
    body_text: String,
    updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(sqlx::FromRow)]
struct SignatureRow {
    id: Uuid,
    mailbox_id: Option<Uuid>,
    name: String,
    body_html: String,
    is_default: bool,
}

#[derive(sqlx::FromRow)]
struct OwnedFile {
    filename: String,
    content_type: String,
    storage_key: String,
    message_id: Option<Uuid>,
    draft_id: Option<Uuid>,
    scan_status: String,
}
