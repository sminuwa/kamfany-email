//! Stored attachment bytes for drafts, sent mail, and received mail.

use sqlx::PgPool;
use uuid::Uuid;

use crate::api::AppError;
use crate::compose::MAX_TOTAL_BYTES;
use crate::providers::{RemoteAttachment, ResendClient};
use crate::storage::BlobStore;

pub struct StoredFile {
    pub filename: String,
    pub content_type: String,
    pub bytes: Vec<u8>,
}

pub async fn read_draft_files(
    pool: &PgPool,
    storage: &BlobStore,
    draft_id: Uuid,
    author_id: Uuid,
    mailbox_id: Uuid,
) -> Result<Vec<StoredFile>, AppError> {
    let rows = sqlx::query_as::<_, FileRow>(
        r#"
        SELECT a.filename, a.content_type, a.size_bytes, a.storage_key
        FROM attachments a
        JOIN drafts d ON d.id = a.draft_id
        WHERE d.id = $1 AND d.author_user_id = $2 AND d.mailbox_id = $3
        ORDER BY a.created_at
        "#,
    )
    .bind(draft_id)
    .bind(author_id)
    .bind(mailbox_id)
    .fetch_all(pool)
    .await?;
    if rows.len() > crate::compose::MAX_FILES {
        return Err(AppError::Validation(
            "A message can have at most 10 attachments.".into(),
        ));
    }
    let total: i64 = rows.iter().map(|row| row.size_bytes).sum();
    if total > MAX_TOTAL_BYTES {
        return Err(AppError::Validation(
            "Attachments can be at most 25 MB together.".into(),
        ));
    }
    let mut files = Vec::with_capacity(rows.len());
    for row in rows {
        let bytes = storage.get(&row.storage_key).await?;
        files.push(StoredFile {
            filename: row.filename,
            content_type: row.content_type,
            bytes,
        });
    }
    Ok(files)
}

pub async fn require_draft(
    pool: &PgPool,
    draft_id: Uuid,
    author_id: Uuid,
    mailbox_id: Uuid,
) -> Result<(), AppError> {
    let found = sqlx::query_scalar::<_, bool>(
        r#"
        SELECT EXISTS(
            SELECT 1 FROM drafts
            WHERE id = $1 AND author_user_id = $2 AND mailbox_id = $3
        )
        "#,
    )
    .bind(draft_id)
    .bind(author_id)
    .bind(mailbox_id)
    .fetch_one(pool)
    .await?;
    if found {
        Ok(())
    } else {
        Err(AppError::NotFound)
    }
}

pub async fn finish_draft(
    pool: &PgPool,
    draft_id: Uuid,
    message_id: Uuid,
) -> Result<(), AppError> {
    let moved = sqlx::query(
        r#"
        UPDATE attachments
        SET message_id = $2, draft_id = NULL
        WHERE draft_id = $1
        "#,
    )
    .bind(draft_id)
    .bind(message_id)
    .execute(pool)
    .await?;
    if moved.rows_affected() > 0 {
        sqlx::query("UPDATE messages SET has_attachments = true WHERE id = $1")
            .bind(message_id)
            .execute(pool)
            .await?;
    }
    sqlx::query("DELETE FROM drafts WHERE id = $1")
        .bind(draft_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn purge_message_files(pool: &PgPool, storage: &BlobStore, message_ids: &[Uuid]) {
    let keys = sqlx::query_scalar::<_, String>(
        "SELECT storage_key FROM attachments WHERE message_id = ANY($1)",
    )
    .bind(message_ids)
    .fetch_all(pool)
    .await
    .unwrap_or_default();
    for key in keys {
        storage.delete(&key).await;
    }
}

pub async fn store_received_files(
    pool: &PgPool,
    storage: &BlobStore,
    client: &ResendClient,
    scanner: Option<&str>,
    provider_message_id: &str,
    files: &[RemoteAttachment],
) -> Result<(), AppError> {
    let messages = sqlx::query_scalar::<_, Uuid>(
        r#"
        SELECT id FROM messages
        WHERE provider = 'resend' AND provider_message_id = $1
        "#,
    )
    .bind(provider_message_id)
    .fetch_all(pool)
    .await?;
    for message_id in messages {
        for file in files {
            if file.size > MAX_TOTAL_BYTES {
                tracing::warn!(
                    message_id = %message_id,
                    filename = %file.filename,
                    "received attachment is larger than the desk limit"
                );
                continue;
            }
            let exists = sqlx::query_scalar::<_, bool>(
                r#"
                SELECT EXISTS(
                    SELECT 1 FROM attachments
                    WHERE message_id = $1 AND provider_attachment_id = $2
                )
                "#,
            )
            .bind(message_id)
            .bind(&file.id)
            .fetch_one(pool)
            .await?;
            if exists {
                continue;
            }
            let Some(url) = file.download_url.as_deref() else {
                continue;
            };
            let bytes = client.download(url).await.map_err(super::provider_error)?;
            if bytes.len() > MAX_TOTAL_BYTES as usize {
                continue;
            }
            let id = Uuid::new_v4();
            let key = format!("attachments/{id}");
            let scan_status = crate::scan::status_for(scanner, &bytes).await;
            storage.put(&key, &bytes).await?;
            let inserted = sqlx::query(
                r#"
                INSERT INTO attachments (
                    id, message_id, filename, content_type, size_bytes, storage_key,
                    provider_attachment_id, scan_status
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
                ON CONFLICT (message_id, provider_attachment_id)
                    WHERE provider_attachment_id IS NOT NULL
                DO NOTHING
                "#,
            )
            .bind(id)
            .bind(message_id)
            .bind(&file.filename)
            .bind(&file.content_type)
            .bind(bytes.len() as i64)
            .bind(&key)
            .bind(&file.id)
            .bind(scan_status)
            .execute(pool)
            .await?;
            if inserted.rows_affected() == 0 {
                storage.delete(&key).await;
            }
        }
    }
    Ok(())
}

#[derive(sqlx::FromRow)]
struct FileRow {
    filename: String,
    content_type: String,
    size_bytes: i64,
    storage_key: String,
}
