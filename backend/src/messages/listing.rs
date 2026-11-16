use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::api::AppError;
use crate::auth::CurrentUser;

use super::access::require_reader;

const PAGE: usize = 50;

#[derive(Debug, Serialize)]
pub struct Person {
    pub name: Option<String>,
    pub address: String,
}

#[derive(Debug, Serialize)]
pub struct MessageItem {
    pub id: Uuid,
    pub thread_id: Option<Uuid>,
    pub thread_count: i64,
    pub direction: String,
    pub status: String,
    pub folder_id: Option<Uuid>,
    pub folder_key: Option<String>,
    pub folder_name: Option<String>,
    pub from_address: String,
    pub from_name: Option<String>,
    pub to: Vec<Person>,
    pub cc: Vec<Person>,
    pub bcc: Vec<Person>,
    pub subject: String,
    pub preview: Option<String>,
    pub activity_at: DateTime<Utc>,
    pub is_read: bool,
    pub is_starred: bool,
    pub is_important: bool,
    pub snoozed_until: Option<DateTime<Utc>>,
    pub has_attachments: bool,
    pub label_ids: Vec<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mailbox_address: Option<String>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct AttachmentItem {
    pub id: Uuid,
    pub filename: String,
    pub content_type: String,
    pub size_bytes: i64,
    pub scan_status: String,
}

#[derive(Debug, Serialize)]
pub struct MessageBody {
    #[serde(flatten)]
    pub item: MessageItem,
    pub body_text: Option<String>,
    pub body_html: Option<String>,
    pub attachments: Vec<AttachmentItem>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct ThreadItem {
    pub id: Uuid,
    pub from_address: String,
    pub from_name: Option<String>,
    pub subject: String,
    pub preview: Option<String>,
    pub activity_at: DateTime<Utc>,
    pub is_read: bool,
}

#[derive(Debug, Serialize)]
pub struct MessagePage {
    pub messages: Vec<MessageItem>,
    pub next_cursor: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct MessageView {
    pub message: MessageBody,
    pub thread: Vec<ThreadItem>,
}

pub struct ListQuery {
    pub view: Option<String>,
    pub folder_id: Option<Uuid>,
    pub cursor: Option<String>,
}

pub async fn list_messages(
    pool: &PgPool,
    mailbox_id: Uuid,
    user: &CurrentUser,
    query: ListQuery,
) -> Result<MessagePage, AppError> {
    require_reader(pool, mailbox_id, user).await?;
    let view = query.view.unwrap_or_else(|| "inbox".to_string());
    if query.folder_id.is_none()
        && !matches!(
            view.as_str(),
            "inbox" | "starred" | "snoozed" | "sent" | "drafts" | "archive" | "spam" | "trash"
        )
    {
        return Err(AppError::Validation(
            "That folder view is not known.".into(),
        ));
    }
    if query.folder_id.is_none() && view == "drafts" {
        return list_drafts(pool, mailbox_id, user.id, query.cursor).await;
    }
    if let Some(folder_id) = query.folder_id {
        let exists = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM folders WHERE id = $1 AND mailbox_id = $2)",
        )
        .bind(folder_id)
        .bind(mailbox_id)
        .fetch_one(pool)
        .await?;
        if !exists {
            return Err(AppError::NotFound);
        }
    }
    let (cursor_at, cursor_id) = parse_cursor(query.cursor.as_deref())?;
    let rows = sqlx::query_as::<_, ItemRow>(
        r#"
        SELECT m.id,
               m.thread_id,
               m.direction,
               m.status,
               f.id AS folder_id,
               f.system_key AS folder_key,
               f.name AS folder_name,
               m.from_address::text AS from_address,
               m.from_name,
               m.subject,
               m.preview,
               COALESCE(m.received_at, m.sent_at, m.created_at) AS activity_at,
               m.is_read,
               m.is_starred,
               m.is_important,
               m.snoozed_until,
               m.has_attachments
        FROM messages m
        LEFT JOIN message_folder_assignments a ON a.message_id = m.id
        LEFT JOIN folders f ON f.id = a.folder_id
        WHERE m.mailbox_id = $1
          AND (
                ($2::uuid IS NOT NULL AND f.id = $2)
                OR (
                    $2 IS NULL AND $3 = 'starred'
                    AND m.is_starred
                    AND COALESCE(f.system_key, '') <> 'trash'
                )
                OR (
                    $2 IS NULL AND $3 = 'snoozed'
                    AND m.snoozed_until > now()
                    AND COALESCE(f.system_key, '') <> 'trash'
                )
                OR (
                    $2 IS NULL AND $3 = 'inbox'
                    AND f.system_key = 'inbox'
                    AND (m.snoozed_until IS NULL OR m.snoozed_until <= now())
                )
                OR (
                    $2 IS NULL
                    AND $3 IN ('sent', 'drafts', 'archive', 'spam', 'trash')
                    AND f.system_key = $3
                )
          )
          AND (
                $4::timestamptz IS NULL
                OR COALESCE(m.received_at, m.sent_at, m.created_at) < $4
                OR (
                    COALESCE(m.received_at, m.sent_at, m.created_at) = $4
                    AND m.id < $5
                )
          )
        ORDER BY COALESCE(m.received_at, m.sent_at, m.created_at) DESC, m.id DESC
        LIMIT $6
        "#,
    )
    .bind(mailbox_id)
    .bind(query.folder_id)
    .bind(&view)
    .bind(cursor_at)
    .bind(cursor_id)
    .bind(i64::try_from(PAGE + 1).unwrap_or(51))
    .fetch_all(pool)
    .await?;
    let mut rows = rows;
    let next_cursor = if rows.len() > PAGE {
        rows.pop();
        rows.last()
            .map(|row| format!("{}|{}", row.activity_at.to_rfc3339(), row.id))
    } else {
        None
    };
    let messages = hydrate(pool, rows).await?;
    Ok(MessagePage {
        messages,
        next_cursor,
    })
}

pub async fn get_message(
    pool: &PgPool,
    message_id: Uuid,
    user: &CurrentUser,
) -> Result<MessageView, AppError> {
    let mailbox_id = super::access::message_mailbox(pool, message_id).await?;
    let grant = require_reader(pool, mailbox_id, user)
        .await
        .map_err(hide_forbidden)?;
    let row = sqlx::query_as::<_, DetailRow>(
        r#"
        SELECT m.id,
               m.thread_id,
               m.direction,
               m.status,
               f.id AS folder_id,
               f.system_key AS folder_key,
               f.name AS folder_name,
               m.from_address::text AS from_address,
               m.from_name,
               m.subject,
               m.preview,
               m.body_text,
               m.body_html,
               COALESCE(m.received_at, m.sent_at, m.created_at) AS activity_at,
               m.is_read,
               m.is_starred,
               m.is_important,
               m.snoozed_until,
               m.has_attachments
        FROM messages m
        LEFT JOIN message_folder_assignments a ON a.message_id = m.id
        LEFT JOIN folders f ON f.id = a.folder_id
        WHERE m.id = $1 AND m.mailbox_id = $2
        "#,
    )
    .bind(message_id)
    .bind(grant.mailbox_id)
    .fetch_optional(pool)
    .await?
    .ok_or(AppError::NotFound)?;
    let mut items = hydrate(pool, vec![row.item.clone()]).await?;
    let item = items.pop().ok_or(AppError::NotFound)?;
    let thread = thread_of(pool, grant.mailbox_id, row.item.thread_id).await?;
    let attachments = sqlx::query_as::<_, AttachmentItem>(
        r#"
        SELECT id, filename, content_type, size_bytes, scan_status
        FROM attachments
        WHERE message_id = $1
        ORDER BY created_at
        "#,
    )
    .bind(message_id)
    .fetch_all(pool)
    .await?;
    Ok(MessageView {
        message: MessageBody {
            item,
            body_text: row.body_text,
            body_html: row.body_html,
            attachments,
        },
        thread,
    })
}

async fn list_drafts(
    pool: &PgPool,
    mailbox_id: Uuid,
    author_id: Uuid,
    cursor: Option<String>,
) -> Result<MessagePage, AppError> {
    let mailbox = sqlx::query_as::<_, MailboxName>(
        "SELECT address::text AS address, display_name FROM mailboxes WHERE id = $1",
    )
    .bind(mailbox_id)
    .fetch_optional(pool)
    .await?
    .ok_or(AppError::NotFound)?;
    let (cursor_at, cursor_id) = parse_cursor(cursor.as_deref())?;
    let rows = sqlx::query_as::<_, DraftListRow>(
        r#"
        SELECT d.id,
               d.subject,
               d.body_text,
               d.updated_at,
               d.to_recipients,
               d.cc_recipients,
               d.bcc_recipients,
               EXISTS(SELECT 1 FROM attachments a WHERE a.draft_id = d.id) AS has_attachments
        FROM drafts d
        WHERE d.mailbox_id = $1
          AND d.author_user_id = $2
          AND (
                $3::timestamptz IS NULL
                OR d.updated_at < $3
                OR (d.updated_at = $3 AND d.id < $4)
          )
        ORDER BY d.updated_at DESC, d.id DESC
        LIMIT $5
        "#,
    )
    .bind(mailbox_id)
    .bind(author_id)
    .bind(cursor_at)
    .bind(cursor_id)
    .bind(i64::try_from(PAGE + 1).unwrap_or(51))
    .fetch_all(pool)
    .await?;
    let mut rows = rows;
    let next_cursor = if rows.len() > PAGE {
        rows.pop();
        rows.last()
            .map(|row| format!("{}|{}", row.updated_at.to_rfc3339(), row.id))
    } else {
        None
    };
    let messages = rows
        .into_iter()
        .map(|row| MessageItem {
            id: row.id,
            thread_id: None,
            thread_count: 1,
            direction: "outbound".to_string(),
            status: "draft".to_string(),
            folder_id: None,
            folder_key: Some("drafts".to_string()),
            folder_name: Some("Drafts".to_string()),
            from_address: mailbox.address.clone(),
            from_name: Some(mailbox.display_name.clone()),
            to: people_from(&row.to_recipients),
            cc: people_from(&row.cc_recipients),
            bcc: people_from(&row.bcc_recipients),
            subject: if row.subject.trim().is_empty() {
                "(no subject)".to_string()
            } else {
                row.subject
            },
            preview: draft_preview(&row.body_text),
            activity_at: row.updated_at,
            is_read: true,
            is_starred: false,
            is_important: false,
            snoozed_until: None,
            has_attachments: row.has_attachments,
            label_ids: Vec::new(),
            mailbox_address: None,
        })
        .collect();
    Ok(MessagePage {
        messages,
        next_cursor,
    })
}

fn draft_preview(text: &str) -> Option<String> {
    let flat = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let shortened: String = flat.chars().take(140).collect();
    (!shortened.is_empty()).then_some(shortened)
}

fn people_from(value: &serde_json::Value) -> Vec<Person> {
    value
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str())
                .filter(|address| !address.is_empty())
                .map(|address| Person {
                    name: None,
                    address: address.to_string(),
                })
                .collect()
        })
        .unwrap_or_default()
}

fn hide_forbidden(error: AppError) -> AppError {
    match error {
        AppError::Forbidden(_) => AppError::NotFound,
        other => other,
    }
}

pub(crate) async fn hydrate(
    pool: &PgPool,
    rows: Vec<ItemRow>,
) -> Result<Vec<MessageItem>, AppError> {
    if rows.is_empty() {
        return Ok(Vec::new());
    }
    let ids: Vec<Uuid> = rows.iter().map(|row| row.id).collect();
    let thread_ids: Vec<Uuid> = rows.iter().filter_map(|row| row.thread_id).collect();
    let recipients = sqlx::query_as::<_, RecipientRow>(
        r#"
        SELECT message_id, kind, address::text AS address, name
        FROM recipients
        WHERE message_id = ANY($1)
        ORDER BY kind, address
        "#,
    )
    .bind(&ids)
    .fetch_all(pool)
    .await?;
    let labels = sqlx::query_as::<_, LabelLink>(
        "SELECT message_id, label_id FROM message_labels WHERE message_id = ANY($1)",
    )
    .bind(&ids)
    .fetch_all(pool)
    .await?;
    let counts = if thread_ids.is_empty() {
        Vec::new()
    } else {
        sqlx::query_as::<_, ThreadCount>(
            r#"
            SELECT thread_id, count(*) AS total
            FROM messages
            WHERE thread_id = ANY($1)
            GROUP BY thread_id
            "#,
        )
        .bind(&thread_ids)
        .fetch_all(pool)
        .await?
    };
    Ok(rows
        .into_iter()
        .map(|row| {
            let mut to = Vec::new();
            let mut cc = Vec::new();
            let mut bcc = Vec::new();
            for recipient in recipients.iter().filter(|item| item.message_id == row.id) {
                let person = Person {
                    name: recipient.name.clone(),
                    address: recipient.address.clone(),
                };
                match recipient.kind.as_str() {
                    "cc" => cc.push(person),
                    "bcc" => bcc.push(person),
                    _ => to.push(person),
                }
            }
            let label_ids = labels
                .iter()
                .filter(|item| item.message_id == row.id)
                .map(|item| item.label_id)
                .collect();
            let thread_count = row
                .thread_id
                .and_then(|id| counts.iter().find(|count| count.thread_id == id))
                .map(|count| count.total)
                .unwrap_or(1);
            MessageItem {
                id: row.id,
                thread_id: row.thread_id,
                thread_count,
                direction: row.direction,
                status: row.status,
                folder_id: row.folder_id,
                folder_key: row.folder_key,
                folder_name: row.folder_name,
                from_address: row.from_address,
                from_name: row.from_name,
                to,
                cc,
                bcc,
                subject: row.subject,
                preview: row.preview,
                activity_at: row.activity_at,
                is_read: row.is_read,
                is_starred: row.is_starred,
                is_important: row.is_important,
                snoozed_until: row.snoozed_until,
                has_attachments: row.has_attachments,
                label_ids,
                mailbox_address: None,
            }
        })
        .collect())
}

async fn thread_of(
    pool: &PgPool,
    mailbox_id: Uuid,
    thread_id: Option<Uuid>,
) -> Result<Vec<ThreadItem>, AppError> {
    let Some(thread_id) = thread_id else {
        return Ok(Vec::new());
    };
    sqlx::query_as::<_, ThreadItem>(
        r#"
        SELECT id,
               from_address::text AS from_address,
               from_name,
               subject,
               preview,
               COALESCE(received_at, sent_at, created_at) AS activity_at,
               is_read
        FROM messages
        WHERE mailbox_id = $1 AND thread_id = $2
        ORDER BY COALESCE(received_at, sent_at, created_at), id
        "#,
    )
    .bind(mailbox_id)
    .bind(thread_id)
    .fetch_all(pool)
    .await
    .map_err(AppError::from)
}

pub(crate) fn parse_cursor(
    value: Option<&str>,
) -> Result<(Option<DateTime<Utc>>, Option<Uuid>), AppError> {
    let Some(value) = value.filter(|item| !item.is_empty()) else {
        return Ok((None, None));
    };
    let Some((stamp, id)) = value.rsplit_once('|') else {
        return Err(AppError::Validation(
            "That page cursor is not valid.".into(),
        ));
    };
    let stamp = DateTime::parse_from_rfc3339(stamp)
        .map_err(|_| AppError::Validation("That page cursor is not valid.".into()))?
        .with_timezone(&Utc);
    let id = Uuid::parse_str(id)
        .map_err(|_| AppError::Validation("That page cursor is not valid.".into()))?;
    Ok((Some(stamp), Some(id)))
}

#[derive(sqlx::FromRow)]
struct DraftListRow {
    id: Uuid,
    subject: String,
    body_text: String,
    updated_at: DateTime<Utc>,
    to_recipients: serde_json::Value,
    cc_recipients: serde_json::Value,
    bcc_recipients: serde_json::Value,
    has_attachments: bool,
}

#[derive(sqlx::FromRow)]
struct MailboxName {
    address: String,
    display_name: String,
}

#[derive(Clone, sqlx::FromRow)]
pub(crate) struct ItemRow {
    pub(crate) id: Uuid,
    pub(crate) thread_id: Option<Uuid>,
    pub(crate) direction: String,
    pub(crate) status: String,
    pub(crate) folder_id: Option<Uuid>,
    pub(crate) folder_key: Option<String>,
    pub(crate) folder_name: Option<String>,
    pub(crate) from_address: String,
    pub(crate) from_name: Option<String>,
    pub(crate) subject: String,
    pub(crate) preview: Option<String>,
    pub(crate) activity_at: DateTime<Utc>,
    pub(crate) is_read: bool,
    pub(crate) is_starred: bool,
    pub(crate) is_important: bool,
    pub(crate) snoozed_until: Option<DateTime<Utc>>,
    pub(crate) has_attachments: bool,
}

#[derive(sqlx::FromRow)]
struct DetailRow {
    #[sqlx(flatten)]
    item: ItemRow,
    body_text: Option<String>,
    body_html: Option<String>,
}

#[derive(sqlx::FromRow)]
struct RecipientRow {
    message_id: Uuid,
    kind: String,
    address: String,
    name: Option<String>,
}

#[derive(sqlx::FromRow)]
struct LabelLink {
    message_id: Uuid,
    label_id: Uuid,
}

#[derive(sqlx::FromRow)]
struct ThreadCount {
    thread_id: Uuid,
    total: i64,
}
