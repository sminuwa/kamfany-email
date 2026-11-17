use chrono::{DateTime, Duration, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::api::AppError;
use crate::auth::{CurrentUser, write_audit};

use super::access::{clean_ids, confirm_messages, require_reader};

pub enum Destination {
    System(String),
    Folder(Uuid),
}

pub struct BulkChange {
    pub ids: Vec<Uuid>,
    pub action: String,
    pub read: Option<bool>,
    pub starred: Option<bool>,
    pub folder_id: Option<Uuid>,
    pub system_key: Option<String>,
    pub label_id: Option<Uuid>,
    pub snoozed_until: Option<DateTime<Utc>>,
}

pub async fn mark_read(
    pool: &PgPool,
    user: &CurrentUser,
    mailbox_id: Uuid,
    ids: Vec<Uuid>,
    read: bool,
) -> Result<(), AppError> {
    let ids = clean_ids(ids)?;
    require_reader(pool, mailbox_id, user).await?;
    confirm_messages(pool, mailbox_id, &ids).await?;
    sqlx::query("UPDATE messages SET is_read = $3 WHERE mailbox_id = $1 AND id = ANY($2)")
        .bind(mailbox_id)
        .bind(&ids)
        .bind(read)
        .execute(pool)
        .await?;
    Ok(())
}

pub(crate) fn accept_wake(until: DateTime<Utc>, now: DateTime<Utc>) -> Result<(), AppError> {
    if until <= now {
        return Err(AppError::Validation("Choose a time in the future.".into()));
    }
    if until > now + Duration::days(366) {
        return Err(AppError::Validation(
            "Snooze can be at most a year ahead.".into(),
        ));
    }
    Ok(())
}

pub async fn mark_snooze(
    pool: &PgPool,
    user: &CurrentUser,
    mailbox_id: Uuid,
    ids: Vec<Uuid>,
    until: Option<DateTime<Utc>>,
) -> Result<(), AppError> {
    let ids = clean_ids(ids)?;
    require_reader(pool, mailbox_id, user).await?;
    confirm_messages(pool, mailbox_id, &ids).await?;
    if let Some(until) = until {
        accept_wake(until, Utc::now())?;
        let blocked: i64 = sqlx::query_scalar(
            r#"
            SELECT count(*)
            FROM messages m
            LEFT JOIN message_folder_assignments a ON a.message_id = m.id
            LEFT JOIN folders f ON f.id = a.folder_id
            WHERE m.mailbox_id = $1
              AND m.id = ANY($2)
              AND (m.status = 'draft' OR f.system_key IS DISTINCT FROM 'inbox')
            "#,
        )
        .bind(mailbox_id)
        .bind(&ids)
        .fetch_one(pool)
        .await?;
        if blocked > 0 {
            return Err(AppError::Validation(
                "Only inbox mail can be snoozed.".into(),
            ));
        }
        sqlx::query(
            "UPDATE messages SET snoozed_until = $3 WHERE mailbox_id = $1 AND id = ANY($2)",
        )
        .bind(mailbox_id)
        .bind(&ids)
        .bind(until)
        .execute(pool)
        .await?;
    } else {
        sqlx::query(
            "UPDATE messages SET snoozed_until = NULL WHERE mailbox_id = $1 AND id = ANY($2)",
        )
        .bind(mailbox_id)
        .bind(&ids)
        .execute(pool)
        .await?;
    }
    Ok(())
}

pub async fn mark_star(
    pool: &PgPool,
    user: &CurrentUser,
    mailbox_id: Uuid,
    ids: Vec<Uuid>,
    starred: bool,
) -> Result<(), AppError> {
    let ids = clean_ids(ids)?;
    require_reader(pool, mailbox_id, user).await?;
    confirm_messages(pool, mailbox_id, &ids).await?;
    sqlx::query("UPDATE messages SET is_starred = $3 WHERE mailbox_id = $1 AND id = ANY($2)")
        .bind(mailbox_id)
        .bind(&ids)
        .bind(starred)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn move_messages(
    pool: &PgPool,
    user: &CurrentUser,
    mailbox_id: Uuid,
    ids: Vec<Uuid>,
    destination: Destination,
) -> Result<(), AppError> {
    let ids = clean_ids(ids)?;
    require_reader(pool, mailbox_id, user).await?;
    confirm_messages(pool, mailbox_id, &ids).await?;
    let folder_id = resolve_folder(pool, mailbox_id, destination).await?;
    let mut tx = pool.begin().await?;
    sqlx::query(
        r#"
        INSERT INTO message_folder_assignments (message_id, folder_id)
        SELECT id, $3
        FROM messages
        WHERE mailbox_id = $1 AND id = ANY($2)
        ON CONFLICT (message_id) DO UPDATE
        SET folder_id = EXCLUDED.folder_id, assigned_at = now()
        "#,
    )
    .bind(mailbox_id)
    .bind(&ids)
    .bind(folder_id)
    .execute(&mut *tx)
    .await?;
    sqlx::query("UPDATE messages SET snoozed_until = NULL WHERE mailbox_id = $1 AND id = ANY($2)")
        .bind(mailbox_id)
        .bind(&ids)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    tracing::info!(%mailbox_id, count = ids.len(), "messages moved");
    Ok(())
}

pub async fn discard_messages(
    pool: &PgPool,
    user: &CurrentUser,
    mailbox_id: Uuid,
    ids: Vec<Uuid>,
    storage: &crate::storage::BlobStore,
) -> Result<(), AppError> {
    let ids = clean_ids(ids)?;
    require_reader(pool, mailbox_id, user).await?;
    confirm_messages(pool, mailbox_id, &ids).await?;
    let trashed = sqlx::query_scalar::<_, Uuid>(
        r#"
        SELECT m.id
        FROM messages m
        JOIN message_folder_assignments a ON a.message_id = m.id
        JOIN folders f ON f.id = a.folder_id
        WHERE m.mailbox_id = $1
          AND m.id = ANY($2)
          AND f.system_key = 'trash'
        "#,
    )
    .bind(mailbox_id)
    .bind(&ids)
    .fetch_all(pool)
    .await?;
    let rest: Vec<Uuid> = ids
        .iter()
        .copied()
        .filter(|id| !trashed.contains(id))
        .collect();
    if !rest.is_empty() {
        move_messages(
            pool,
            user,
            mailbox_id,
            rest,
            Destination::System("trash".to_string()),
        )
        .await?;
    }
    if !trashed.is_empty() {
        crate::delivery::purge_message_files(pool, storage, &trashed).await;
        sqlx::query("DELETE FROM messages WHERE mailbox_id = $1 AND id = ANY($2)")
            .bind(mailbox_id)
            .bind(&trashed)
            .execute(pool)
            .await?;
        tracing::info!(%mailbox_id, count = trashed.len(), "messages removed");
        if let Err(error) = write_audit(
            pool,
            Some(user.id),
            "message.deleted",
            Some("mailbox"),
            Some(mailbox_id),
            serde_json::json!({ "count": trashed.len() }),
        )
        .await
        {
            tracing::error!(%error, "audit log was not written");
        }
    }
    Ok(())
}

pub async fn add_label(
    pool: &PgPool,
    user: &CurrentUser,
    mailbox_id: Uuid,
    ids: Vec<Uuid>,
    label_id: Uuid,
) -> Result<(), AppError> {
    let ids = clean_ids(ids)?;
    require_reader(pool, mailbox_id, user).await?;
    confirm_messages(pool, mailbox_id, &ids).await?;
    confirm_label(pool, mailbox_id, label_id).await?;
    sqlx::query(
        r#"
        INSERT INTO message_labels (message_id, label_id)
        SELECT id, $3
        FROM messages
        WHERE mailbox_id = $1 AND id = ANY($2)
        ON CONFLICT DO NOTHING
        "#,
    )
    .bind(mailbox_id)
    .bind(&ids)
    .bind(label_id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn remove_label(
    pool: &PgPool,
    user: &CurrentUser,
    mailbox_id: Uuid,
    ids: Vec<Uuid>,
    label_id: Uuid,
) -> Result<(), AppError> {
    let ids = clean_ids(ids)?;
    require_reader(pool, mailbox_id, user).await?;
    confirm_messages(pool, mailbox_id, &ids).await?;
    sqlx::query("DELETE FROM message_labels WHERE label_id = $1 AND message_id = ANY($2)")
        .bind(label_id)
        .bind(&ids)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn bulk(
    pool: &PgPool,
    user: &CurrentUser,
    mailbox_id: Uuid,
    change: BulkChange,
    storage: &crate::storage::BlobStore,
) -> Result<(), AppError> {
    match change.action.as_str() {
        "read" => {
            mark_read(
                pool,
                user,
                mailbox_id,
                change.ids,
                change.read.unwrap_or(true),
            )
            .await
        }
        "star" => {
            mark_star(
                pool,
                user,
                mailbox_id,
                change.ids,
                change.starred.unwrap_or(true),
            )
            .await
        }
        "snooze" => mark_snooze(pool, user, mailbox_id, change.ids, change.snoozed_until).await,
        "move" => {
            let destination = match (change.folder_id, change.system_key) {
                (Some(folder_id), None) => Destination::Folder(folder_id),
                (None, Some(key)) => Destination::System(key),
                _ => {
                    return Err(AppError::Validation("Choose a folder to move to.".into()));
                }
            };
            move_messages(pool, user, mailbox_id, change.ids, destination).await
        }
        "trash" => discard_messages(pool, user, mailbox_id, change.ids, storage).await,
        "label" => {
            let label_id = change
                .label_id
                .ok_or_else(|| AppError::Validation("Choose a label.".into()))?;
            add_label(pool, user, mailbox_id, change.ids, label_id).await
        }
        "unlabel" => {
            let label_id = change
                .label_id
                .ok_or_else(|| AppError::Validation("Choose a label.".into()))?;
            remove_label(pool, user, mailbox_id, change.ids, label_id).await
        }
        _ => Err(AppError::Validation(
            "That message action is not known.".into(),
        )),
    }
}

async fn resolve_folder(
    pool: &PgPool,
    mailbox_id: Uuid,
    destination: Destination,
) -> Result<Uuid, AppError> {
    let (folder_id, system_key) = match destination {
        Destination::Folder(id) => {
            let key = sqlx::query_scalar::<_, Option<String>>(
                "SELECT system_key FROM folders WHERE id = $1 AND mailbox_id = $2",
            )
            .bind(id)
            .bind(mailbox_id)
            .fetch_optional(pool)
            .await?;
            let Some(key) = key else {
                return Err(AppError::NotFound);
            };
            (id, key)
        }
        Destination::System(key) => {
            if !matches!(
                key.as_str(),
                "inbox" | "sent" | "drafts" | "archive" | "spam" | "trash"
            ) {
                return Err(AppError::Validation(
                    "Choose inbox, sent, drafts, archive, spam, or trash.".into(),
                ));
            }
            let id = sqlx::query_scalar::<_, Uuid>(
                "SELECT id FROM folders WHERE mailbox_id = $1 AND system_key = $2",
            )
            .bind(mailbox_id)
            .bind(&key)
            .fetch_optional(pool)
            .await?;
            let Some(id) = id else {
                return Err(AppError::NotFound);
            };
            (id, Some(key))
        }
    };
    if matches!(system_key.as_deref(), Some("starred" | "snoozed")) {
        return Err(AppError::Validation(
            "Starred and snoozed are flags, not folders.".into(),
        ));
    }
    Ok(folder_id)
}

async fn confirm_label(pool: &PgPool, mailbox_id: Uuid, label_id: Uuid) -> Result<(), AppError> {
    let found = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM labels WHERE id = $1 AND mailbox_id = $2)",
    )
    .bind(label_id)
    .bind(mailbox_id)
    .fetch_one(pool)
    .await?;
    if found {
        Ok(())
    } else {
        Err(AppError::NotFound)
    }
}

#[cfg(test)]
mod tests {
    use super::accept_wake;
    use chrono::{Duration, Utc};

    #[test]
    fn a_wake_time_must_be_in_the_future_and_within_a_year() {
        let now = Utc::now();
        assert!(accept_wake(now, now).is_err());
        assert!(accept_wake(now - Duration::hours(1), now).is_err());
        assert!(accept_wake(now + Duration::hours(2), now).is_ok());
        assert!(accept_wake(now + Duration::days(400), now).is_err());
    }
}
