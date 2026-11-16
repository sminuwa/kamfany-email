//! Mailbox folders and threads shared by storage and the desk.
//!
//! Starred and snoozed stay message flags. Their folder rows exist so the
//! desk can name them. A message is not filed only into those rows.

use sqlx::PgConnection;
use uuid::Uuid;

use crate::api::AppError;

const SYSTEM_FOLDERS: &[(&str, &str, i32)] = &[
    ("inbox", "Inbox", 0),
    ("starred", "Starred", 1),
    ("snoozed", "Snoozed", 2),
    ("sent", "Sent", 3),
    ("drafts", "Drafts", 4),
    ("archive", "Archive", 5),
    ("spam", "Spam", 6),
    ("trash", "Trash", 7),
];

pub async fn ensure_system_folders(
    connection: &mut PgConnection,
    mailbox_id: Uuid,
) -> Result<(), AppError> {
    for (key, name, sort_order) in SYSTEM_FOLDERS {
        sqlx::query(
            r#"
            INSERT INTO folders (mailbox_id, system_key, name, sort_order)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (mailbox_id, system_key) WHERE system_key IS NOT NULL DO NOTHING
            "#,
        )
        .bind(mailbox_id)
        .bind(key)
        .bind(name)
        .bind(sort_order)
        .execute(&mut *connection)
        .await?;
    }
    Ok(())
}

pub async fn assign_system_folder(
    connection: &mut PgConnection,
    mailbox_id: Uuid,
    message_id: Uuid,
    system_key: &str,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        INSERT INTO message_folder_assignments (message_id, folder_id)
        SELECT $1, id
        FROM folders
        WHERE mailbox_id = $2 AND system_key = $3
        ON CONFLICT (message_id) DO UPDATE
        SET folder_id = EXCLUDED.folder_id, assigned_at = now()
        "#,
    )
    .bind(message_id)
    .bind(mailbox_id)
    .bind(system_key)
    .execute(&mut *connection)
    .await?;
    Ok(())
}

pub fn header_tokens(in_reply_to: Option<&str>, references_header: Option<&str>) -> Vec<String> {
    let mut tokens = Vec::new();
    for value in [in_reply_to, references_header].into_iter().flatten() {
        for token in value.split_whitespace() {
            let token = token.trim();
            if (3..998).contains(&token.len()) {
                tokens.push(token.to_string());
            }
        }
    }
    tokens.sort();
    tokens.dedup();
    tokens
}

pub async fn attach_thread(
    connection: &mut PgConnection,
    mailbox_id: Uuid,
    message_id: Uuid,
    subject: &str,
    in_reply_to: Option<&str>,
    references_header: Option<&str>,
    existing_thread: Option<Uuid>,
) -> Result<Uuid, AppError> {
    let thread_id = if let Some(thread_id) = existing_thread {
        let found = sqlx::query_scalar::<_, Uuid>(
            "SELECT id FROM threads WHERE id = $1 AND mailbox_id = $2",
        )
        .bind(thread_id)
        .bind(mailbox_id)
        .fetch_optional(&mut *connection)
        .await?;
        found.ok_or_else(|| AppError::Validation("That thread is not in this mailbox.".into()))?
    } else {
        let tokens = header_tokens(in_reply_to, references_header);
        let matched = if tokens.is_empty() {
            None
        } else {
            sqlx::query_scalar::<_, Uuid>(
                r#"
                SELECT thread_id
                FROM messages
                WHERE mailbox_id = $1
                  AND id <> $2
                  AND thread_id IS NOT NULL
                  AND message_id_header = ANY($3)
                ORDER BY created_at
                LIMIT 1
                "#,
            )
            .bind(mailbox_id)
            .bind(message_id)
            .bind(&tokens)
            .fetch_optional(&mut *connection)
            .await?
        };
        if let Some(thread_id) = matched {
            thread_id
        } else {
            sqlx::query_scalar::<_, Uuid>(
                r#"
                INSERT INTO threads (mailbox_id, subject, last_message_at)
                VALUES ($1, $2, now())
                RETURNING id
                "#,
            )
            .bind(mailbox_id)
            .bind(subject)
            .fetch_one(&mut *connection)
            .await?
        }
    };

    sqlx::query("UPDATE messages SET thread_id = $2 WHERE id = $1")
        .bind(message_id)
        .bind(thread_id)
        .execute(&mut *connection)
        .await?;
    sqlx::query("UPDATE threads SET last_message_at = now() WHERE id = $1")
        .bind(thread_id)
        .execute(&mut *connection)
        .await?;
    Ok(thread_id)
}

#[cfg(test)]
mod tests {
    use super::header_tokens;

    #[test]
    fn header_tokens_keep_message_ids() {
        let tokens = header_tokens(Some("<a@ex>"), Some("<a@ex> <b@ex>"));
        assert_eq!(tokens, vec!["<a@ex>".to_string(), "<b@ex>".to_string()]);
    }
}
