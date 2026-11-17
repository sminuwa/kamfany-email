use sqlx::FromRow;
use sqlx::PgPool;
use uuid::Uuid;

use crate::api::AppError;
use crate::auth::{normalize_email, pool_of, write_audit};
use crate::providers::{EmailProvider, OutboundEmail};
use crate::state::AppState;

use super::{preview, provider_error};

const MAX_RECIPIENTS: usize = 50;

pub struct SendLink {
    pub thread_id: Option<Uuid>,
    pub in_reply_to: Option<String>,
    pub references_header: Option<String>,
    /// A forward keeps the reply headers and starts its own thread.
    pub fresh: bool,
}

pub struct SendRequest {
    pub mailbox_id: Uuid,
    pub to: Vec<String>,
    pub cc: Vec<String>,
    pub bcc: Vec<String>,
    pub subject: String,
    pub text: Option<String>,
    pub html: Option<String>,
    pub link: Option<SendLink>,
    pub draft_id: Option<Uuid>,
}

pub struct SentMessage {
    pub id: Uuid,
    pub status: String,
    pub provider_message_id: String,
}

#[derive(FromRow)]
struct MailboxSend {
    id: Uuid,
    address: String,
    display_name: String,
    is_enabled: bool,
    can_send: Option<bool>,
}

pub async fn send_mail(
    state: &AppState,
    actor: Uuid,
    platform_manager: bool,
    request: SendRequest,
) -> Result<SentMessage, AppError> {
    let pool = pool_of(state)?;
    let client = state
        .provider
        .as_ref()
        .ok_or_else(|| AppError::Unavailable("Resend is not configured.".into()))?;
    let mailbox = load_sender(pool, request.mailbox_id, actor, platform_manager).await?;
    if !mailbox.is_enabled {
        return Err(AppError::Validation("This mailbox is disabled.".into()));
    }
    let domain = mailbox
        .address
        .split_once('@')
        .map(|(_, domain)| domain)
        .unwrap_or_default();
    if !domain.eq_ignore_ascii_case(&state.config.resend.sending_domain) {
        return Err(AppError::Validation(format!(
            "Sending is limited to @{}.",
            state.config.resend.sending_domain
        )));
    }
    let to = normalize_many(&request.to, "To")?;
    let cc = normalize_many(&request.cc, "Cc")?;
    let bcc = normalize_many(&request.bcc, "Bcc")?;
    if to.is_empty() {
        return Err(AppError::Validation("Add at least one recipient.".into()));
    }
    let fanout = crate::groups::fanout(pool, &to, &cc, &bcc).await?;
    if fanout.to.len() + fanout.cc.len() + fanout.bcc.len() > MAX_RECIPIENTS {
        return Err(AppError::Validation(
            "A message can have at most 50 recipients.".into(),
        ));
    }
    let subject = request.subject.trim();
    if subject.is_empty() || subject.chars().count() > 998 || subject.chars().any(char::is_control)
    {
        return Err(AppError::Validation(
            "Enter a subject of 1 to 998 characters.".into(),
        ));
    }
    let (text, html) = crate::compose::message_content(request.text, request.html)?;
    if text.is_none() && html.is_none() {
        return Err(AppError::Validation(
            "A message needs plain text or HTML.".into(),
        ));
    }
    if let Some(draft_id) = request.draft_id {
        super::files::require_draft(pool, draft_id, actor, mailbox.id).await?;
        crate::scan::gate_draft(
            pool,
            &state.storage,
            state.config.scanner.as_deref(),
            draft_id,
        )
        .await?;
    }
    crate::limit::charge(pool, &format!("send:{actor}"), 3600, 30).await?;
    let files = match request.draft_id {
        Some(draft_id) => {
            super::files::read_draft_files(pool, &state.storage, draft_id, actor, mailbox.id)
                .await?
        }
        None => Vec::new(),
    };

    let outgoing = insert_outgoing(
        pool,
        &mailbox,
        &OutgoingDraft {
            subject,
            text: text.as_deref(),
            html: html.as_deref(),
            to: &to,
            cc: &cc,
            bcc: &bcc,
            link: request.link.as_ref(),
        },
    )
    .await?;
    let message_id = outgoing.id;
    let mut headers = Vec::new();
    if let Some(link) = &request.link {
        if let Some(value) = link.in_reply_to.as_deref() {
            headers.push(("In-Reply-To".to_string(), value.to_string()));
        }
        if let Some(value) = link.references_header.as_deref() {
            headers.push(("References".to_string(), value.to_string()));
        }
    }
    match client
        .send_email(&OutboundEmail {
            from_name: mailbox.display_name.clone(),
            from_address: mailbox.address.clone(),
            to: fanout.to,
            cc: fanout.cc,
            bcc: fanout.bcc,
            subject: subject.to_string(),
            text: text.clone(),
            html: html.clone(),
            idempotency_key: message_id.to_string(),
            headers,
            attachments: files
                .iter()
                .map(|file| crate::providers::OutboundAttachment {
                    filename: file.filename.clone(),
                    content_type: file.content_type.clone(),
                    content: file.bytes.clone(),
                })
                .collect(),
        })
        .await
    {
        Ok(receipt) => {
            sqlx::query(
                r#"
                UPDATE messages
                SET status = 'accepted', provider_message_id = $2, sent_at = COALESCE(sent_at, now())
                WHERE id = $1
                  AND status IN ('queued', 'sending')
                "#,
            )
            .bind(message_id)
            .bind(&receipt.provider_message_id)
            .execute(pool)
            .await?;
            if let Some(draft_id) = request.draft_id {
                super::files::finish_draft(pool, draft_id, message_id).await?;
            }
            let copies = files
                .iter()
                .map(|file| GroupFile {
                    filename: file.filename.clone(),
                    content_type: file.content_type.clone(),
                    bytes: file.bytes.clone(),
                    scan_status: file.scan_status.clone(),
                })
                .collect::<Vec<_>>();
            deliver_group_mail(
                state,
                &GroupDelivery {
                    from_address: &mailbox.address,
                    from_name: &mailbox.display_name,
                    subject,
                    text: text.as_deref(),
                    html: html.as_deref(),
                    to: &to,
                    cc: &cc,
                    message_id_header: &outgoing.message_id_header,
                    provider_message_id: &receipt.provider_message_id,
                    mailbox_ids: &fanout.mailboxes,
                    files: &copies,
                },
            )
            .await?;
            write_audit(
                pool,
                Some(actor),
                "mail.accepted",
                Some("message"),
                Some(message_id),
                serde_json::json!({
                    "mailbox_id": mailbox.id,
                    "provider_message_id": receipt.provider_message_id,
                }),
            )
            .await?;
            Ok(SentMessage {
                id: message_id,
                status: "accepted".to_string(),
                provider_message_id: receipt.provider_message_id,
            })
        }
        Err(error) => {
            sqlx::query("UPDATE messages SET status = 'failed' WHERE id = $1")
                .bind(message_id)
                .execute(pool)
                .await?;
            write_audit(
                pool,
                Some(actor),
                "mail.failed",
                Some("message"),
                Some(message_id),
                serde_json::json!({ "mailbox_id": mailbox.id }),
            )
            .await?;
            Err(provider_error(error))
        }
    }
}

fn normalize_many(values: &[String], label: &str) -> Result<Vec<String>, AppError> {
    let mut addresses = Vec::with_capacity(values.len());
    for value in values {
        if value.trim().is_empty() {
            continue;
        }
        addresses.push(normalize_email(value).map_err(|_| {
            AppError::Validation(format!("{label} contains an invalid email address."))
        })?);
    }
    Ok(addresses)
}

async fn load_sender(
    pool: &PgPool,
    mailbox_id: Uuid,
    user_id: Uuid,
    platform_manager: bool,
) -> Result<MailboxSend, AppError> {
    let row = sqlx::query_as::<_, MailboxSend>(
        r#"
        SELECT m.id,
               m.address::text AS address,
               m.display_name,
               m.is_enabled,
               mu.can_send
        FROM mailboxes m
        LEFT JOIN mailbox_users mu ON mu.mailbox_id = m.id AND mu.user_id = $2
        WHERE m.id = $1
        "#,
    )
    .bind(mailbox_id)
    .bind(user_id)
    .fetch_optional(pool)
    .await?;
    let Some(row) = row else {
        return Err(AppError::NotFound);
    };
    if row.can_send.is_none() && !platform_manager {
        return Err(AppError::NotFound);
    }
    if row.can_send != Some(true) {
        return Err(AppError::Forbidden(
            "Sending is not granted for this mailbox.".into(),
        ));
    }
    Ok(row)
}

struct OutgoingDraft<'a> {
    subject: &'a str,
    text: Option<&'a str>,
    html: Option<&'a str>,
    to: &'a [String],
    cc: &'a [String],
    bcc: &'a [String],
    link: Option<&'a SendLink>,
}

struct OutgoingRecord {
    id: Uuid,
    message_id_header: String,
}

async fn insert_outgoing(
    pool: &PgPool,
    mailbox: &MailboxSend,
    draft: &OutgoingDraft<'_>,
) -> Result<OutgoingRecord, AppError> {
    let mut tx = pool.begin().await?;
    crate::records::ensure_system_folders(&mut tx, mailbox.id).await?;
    let id = Uuid::new_v4();
    let domain = mailbox
        .address
        .split_once('@')
        .map(|(_, domain)| domain)
        .unwrap_or("kamfany.mail");
    let message_header = format!("<{id}@{domain}>");
    let link = draft.link;
    sqlx::query(
        r#"
        INSERT INTO messages (
            id, mailbox_id, provider, direction, status, from_address, from_name,
            subject, preview, body_text, body_html, is_read, sent_at,
            message_id_header, in_reply_to, references_header
        )
        VALUES (
            $1, $2, 'resend', 'outbound', 'sending', $3, $4,
            $5, $6, $7, $8, true, now(),
            $9, $10, $11
        )
        "#,
    )
    .bind(id)
    .bind(mailbox.id)
    .bind(&mailbox.address)
    .bind(&mailbox.display_name)
    .bind(draft.subject)
    .bind(draft.text.and_then(preview))
    .bind(draft.text)
    .bind(draft.html)
    .bind(&message_header)
    .bind(link.and_then(|item| item.in_reply_to.as_deref()))
    .bind(link.and_then(|item| item.references_header.as_deref()))
    .execute(&mut *tx)
    .await?;
    for (kind, addresses) in [("to", draft.to), ("cc", draft.cc), ("bcc", draft.bcc)] {
        for address in addresses {
            sqlx::query("INSERT INTO recipients (message_id, kind, address) VALUES ($1, $2, $3)")
                .bind(id)
                .bind(kind)
                .bind(address)
                .execute(&mut *tx)
                .await?;
        }
    }
    crate::records::assign_system_folder(&mut tx, mailbox.id, id, "sent").await?;
    let existing_thread = link.and_then(|item| item.thread_id);
    let match_headers = link.is_some_and(|item| !item.fresh && existing_thread.is_none());
    crate::records::attach_thread(
        &mut tx,
        mailbox.id,
        id,
        draft.subject,
        match_headers
            .then(|| link.and_then(|item| item.in_reply_to.as_deref()))
            .flatten(),
        match_headers
            .then(|| link.and_then(|item| item.references_header.as_deref()))
            .flatten(),
        existing_thread,
    )
    .await?;
    tx.commit().await?;
    Ok(OutgoingRecord {
        id,
        message_id_header: message_header,
    })
}

pub struct GroupFile {
    pub filename: String,
    pub content_type: String,
    pub bytes: Vec<u8>,
    pub scan_status: String,
}

pub struct GroupDelivery<'a> {
    pub from_address: &'a str,
    pub from_name: &'a str,
    pub subject: &'a str,
    pub text: Option<&'a str>,
    pub html: Option<&'a str>,
    pub to: &'a [String],
    pub cc: &'a [String],
    pub message_id_header: &'a str,
    pub provider_message_id: &'a str,
    pub mailbox_ids: &'a [Uuid],
    pub files: &'a [GroupFile],
}

pub async fn deliver_group_mail(
    state: &crate::state::AppState,
    delivery: &GroupDelivery<'_>,
) -> Result<(), AppError> {
    if delivery.mailbox_ids.is_empty() {
        return Ok(());
    }
    let pool = pool_of(state)?;
    for mailbox_id in delivery.mailbox_ids {
        let arrival = insert_group_copy(pool, &state.storage, *mailbox_id, delivery).await?;
        if let Some(arrival) = arrival {
            crate::live::announce(state, &arrival).await?;
        }
    }
    Ok(())
}

async fn insert_group_copy(
    pool: &PgPool,
    storage: &crate::storage::BlobStore,
    mailbox_id: Uuid,
    delivery: &GroupDelivery<'_>,
) -> Result<Option<crate::live::Arrival>, AppError> {
    let mut tx = pool.begin().await?;
    crate::records::ensure_system_folders(&mut tx, mailbox_id).await?;
    let message_id = sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO messages (
            mailbox_id, provider, provider_message_id, direction, status,
            from_address, from_name, subject, preview, body_text, body_html,
            received_at, is_read, has_attachments, message_id_header
        )
        VALUES (
            $1, 'resend', $2, 'inbound', 'received',
            $3, $4, $5, $6, $7, $8,
            now(), false, $9, $10
        )
        ON CONFLICT (mailbox_id, provider, provider_message_id)
            WHERE provider_message_id IS NOT NULL
        DO NOTHING
        RETURNING id
        "#,
    )
    .bind(mailbox_id)
    .bind(delivery.provider_message_id)
    .bind(delivery.from_address)
    .bind(delivery.from_name)
    .bind(delivery.subject)
    .bind(delivery.text.and_then(preview))
    .bind(delivery.text)
    .bind(delivery.html)
    .bind(!delivery.files.is_empty())
    .bind(delivery.message_id_header)
    .fetch_optional(&mut *tx)
    .await?;
    let Some(message_id) = message_id else {
        tx.commit().await?;
        return Ok(None);
    };
    for (kind, addresses) in [("to", delivery.to), ("cc", delivery.cc)] {
        for address in addresses {
            sqlx::query("INSERT INTO recipients (message_id, kind, address) VALUES ($1, $2, $3)")
                .bind(message_id)
                .bind(kind)
                .bind(address)
                .execute(&mut *tx)
                .await?;
        }
    }
    crate::records::assign_system_folder(&mut tx, mailbox_id, message_id, "inbox").await?;
    crate::records::attach_thread(
        &mut tx,
        mailbox_id,
        message_id,
        delivery.subject,
        None,
        None,
        None,
    )
    .await?;
    let mut stored_keys = Vec::new();
    for file in delivery.files {
        let id = Uuid::new_v4();
        let key = format!("attachments/{id}");
        storage.put(&key, &file.bytes).await?;
        stored_keys.push(key.clone());
        let scan_status = if matches!(
            file.scan_status.as_str(),
            "pending" | "clean" | "rejected" | "failed"
        ) {
            file.scan_status.as_str()
        } else {
            "pending"
        };
        if let Err(error) = sqlx::query(
            r#"
            INSERT INTO attachments (
                id, message_id, filename, content_type, size_bytes, storage_key, scan_status
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
        )
        .bind(id)
        .bind(message_id)
        .bind(&file.filename)
        .bind(&file.content_type)
        .bind(file.bytes.len() as i64)
        .bind(&key)
        .bind(scan_status)
        .execute(&mut *tx)
        .await
        {
            for key in stored_keys {
                storage.delete(&key).await;
            }
            return Err(error.into());
        }
    }
    tx.commit().await?;
    Ok(Some(crate::live::Arrival {
        mailbox_id,
        message_id,
        from_name: Some(delivery.from_name.to_string()),
        from_address: delivery.from_address.to_string(),
        subject: delivery.subject.to_string(),
    }))
}
