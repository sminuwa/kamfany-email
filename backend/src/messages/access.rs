use sqlx::PgPool;
use uuid::Uuid;

use crate::api::AppError;
use crate::auth::CurrentUser;

pub struct MailGrant {
    pub mailbox_id: Uuid,
    pub can_send: bool,
}

pub async fn require_reader(
    pool: &PgPool,
    mailbox_id: Uuid,
    user: &CurrentUser,
) -> Result<MailGrant, AppError> {
    let row = sqlx::query_as::<_, GrantRow>(
        r#"
        SELECT mu.can_read, mu.can_send
        FROM mailboxes m
        LEFT JOIN mailbox_users mu ON mu.mailbox_id = m.id AND mu.user_id = $2
        WHERE m.id = $1
        "#,
    )
    .bind(mailbox_id)
    .bind(user.id)
    .fetch_optional(pool)
    .await?;
    let Some(row) = row else {
        return Err(AppError::NotFound);
    };
    if row.can_read.is_none() && !user.allows("mailbox.manage") {
        return Err(AppError::NotFound);
    }
    if row.can_read != Some(true) {
        return Err(AppError::Forbidden("You cannot read this mailbox.".into()));
    }
    Ok(MailGrant {
        mailbox_id,
        can_send: row.can_send == Some(true),
    })
}

pub async fn message_mailbox(pool: &PgPool, message_id: Uuid) -> Result<Uuid, AppError> {
    sqlx::query_scalar::<_, Uuid>("SELECT mailbox_id FROM messages WHERE id = $1")
        .bind(message_id)
        .fetch_optional(pool)
        .await?
        .ok_or(AppError::NotFound)
}

pub fn clean_ids(ids: Vec<Uuid>) -> Result<Vec<Uuid>, AppError> {
    let mut ids = ids;
    ids.sort();
    ids.dedup();
    if ids.is_empty() {
        return Err(AppError::Validation("Choose a message.".into()));
    }
    if ids.len() > 50 {
        return Err(AppError::Validation("Choose at most 50 messages.".into()));
    }
    Ok(ids)
}

pub async fn confirm_messages(
    pool: &PgPool,
    mailbox_id: Uuid,
    ids: &[Uuid],
) -> Result<(), AppError> {
    let count = sqlx::query_scalar::<_, i64>(
        "SELECT count(*) FROM messages WHERE mailbox_id = $1 AND id = ANY($2)",
    )
    .bind(mailbox_id)
    .bind(ids)
    .fetch_one(pool)
    .await?;
    if count != i64::try_from(ids.len()).unwrap_or(i64::MAX) {
        return Err(AppError::NotFound);
    }
    Ok(())
}

#[derive(sqlx::FromRow)]
struct GrantRow {
    can_read: Option<bool>,
    can_send: Option<bool>,
}
