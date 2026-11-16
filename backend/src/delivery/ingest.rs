use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

use crate::api::AppError;
use crate::auth::normalize_email;
use crate::live::Arrival;
use crate::providers::InboundEmail;

use super::preview;

pub async fn store_inbound(
    pool: &PgPool,
    email: &InboundEmail,
    extra_recipients: &[String],
) -> Result<Vec<Arrival>, AppError> {
    let mut addresses = email.to.clone();
    addresses.extend(email.cc.iter().cloned());
    addresses.extend(extra_recipients.iter().cloned());
    let mut normalized = Vec::new();
    for address in addresses {
        if let Ok(address) = normalize_email(&address) {
            normalized.push(address);
        }
    }
    normalized.sort();
    normalized.dedup();
    if normalized.is_empty() {
        return Ok(Vec::new());
    }

    let mailboxes = sqlx::query_as::<_, MailboxMatch>(
        "SELECT id FROM mailboxes WHERE lower(address::text) = ANY($1)",
    )
    .bind(&normalized)
    .fetch_all(pool)
    .await?;

    let mut stored = Vec::new();
    for mailbox in mailboxes {
        if let Some(arrival) = insert_copy(pool, mailbox.id, email).await? {
            stored.push(arrival);
        }
    }
    Ok(stored)
}

async fn insert_copy(
    pool: &PgPool,
    mailbox_id: Uuid,
    email: &InboundEmail,
) -> Result<Option<Arrival>, AppError> {
    let mut tx = pool.begin().await?;
    let inserted = sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO messages (
            mailbox_id, provider, provider_message_id, direction, status,
            from_address, from_name, subject, preview, body_text, body_html,
            received_at, is_read, has_attachments, message_id_header, in_reply_to, references_header
        )
        VALUES (
            $1, 'resend', $2, 'inbound', 'received',
            $3, $4, $5, $6, $7, $8,
            $9, false, $10, $11, $12, $13
        )
        ON CONFLICT (mailbox_id, provider, provider_message_id)
            WHERE provider_message_id IS NOT NULL
        DO NOTHING
        RETURNING id
        "#,
    )
    .bind(mailbox_id)
    .bind(&email.provider_message_id)
    .bind(&email.from_address)
    .bind(&email.from_name)
    .bind(&email.subject)
    .bind(email.text.as_deref().and_then(preview))
    .bind(email.text.as_deref().map(limit_body))
    .bind(email.html.as_deref().map(limit_body))
    .bind(email.received_at.unwrap_or_else(Utc::now))
    .bind(email.attachment_count > 0)
    .bind(&email.message_id_header)
    .bind(&email.in_reply_to)
    .bind(&email.references_header)
    .fetch_optional(&mut *tx)
    .await?;

    if let Some(message_id) = inserted {
        let folder_id = sqlx::query_scalar::<_, Uuid>(
            "SELECT id FROM folders WHERE mailbox_id = $1 AND system_key = 'inbox'",
        )
        .bind(mailbox_id)
        .fetch_optional(&mut *tx)
        .await?;
        if let Some(folder_id) = folder_id {
            sqlx::query(
                "INSERT INTO message_folder_assignments (message_id, folder_id) VALUES ($1, $2)",
            )
            .bind(message_id)
            .bind(folder_id)
            .execute(&mut *tx)
            .await?;
        }
        for (kind, addresses) in [("to", &email.to), ("cc", &email.cc)] {
            for address in addresses {
                if let Ok(address) = normalize_email(address) {
                    sqlx::query(
                        "INSERT INTO recipients (message_id, kind, address) VALUES ($1, $2, $3)",
                    )
                    .bind(message_id)
                    .bind(kind)
                    .bind(address)
                    .execute(&mut *tx)
                    .await?;
                }
            }
        }
        crate::records::attach_thread(
            &mut tx,
            mailbox_id,
            message_id,
            &email.subject,
            email.in_reply_to.as_deref(),
            email.references_header.as_deref(),
            None,
        )
        .await?;
    }

    sqlx::query(
        r#"
        UPDATE mailboxes
        SET is_receiving_verified = true, verified_at = COALESCE(verified_at, now())
        WHERE id = $1
        "#,
    )
    .bind(mailbox_id)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(inserted.map(|message_id| Arrival {
        mailbox_id,
        message_id,
        from_name: email.from_name.clone(),
        from_address: email.from_address.clone(),
        subject: email.subject.clone(),
    }))
}

fn limit_body(value: &str) -> String {
    value.chars().take(500_000).collect()
}

#[derive(sqlx::FromRow)]
struct MailboxMatch {
    id: Uuid,
}

pub async fn apply_delivery_status(
    pool: &PgPool,
    provider_message_id: &str,
    status: &str,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        UPDATE messages
        SET status = $2
        WHERE provider = 'resend'
          AND provider_message_id = $1
          AND status IN ('queued', 'sending', 'accepted')
        "#,
    )
    .bind(provider_message_id)
    .bind(status)
    .execute(pool)
    .await?;
    Ok(())
}

pub fn delivery_status(event_type: &str) -> Option<&'static str> {
    match event_type {
        "email.delivered" => Some("delivered"),
        "email.bounced" => Some("bounced"),
        "email.failed" => Some("failed"),
        _ => None,
    }
}
