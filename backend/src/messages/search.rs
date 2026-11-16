//! Mail search.
//!
//! Full text uses the generated `search_vector`. The other filters stay in
//! this module so the listing query does not have to grow with them.

use chrono::{DateTime, NaiveDate, Utc};
use uuid::Uuid;

use crate::api::AppError;
use crate::auth::CurrentUser;

use super::access::require_reader;
use super::listing::{ItemRow, MessagePage, hydrate};

const PAGE: i64 = 50;

pub struct SearchQuery {
    pub q: Option<String>,
    pub from: Option<String>,
    pub to: Option<String>,
    pub subject: Option<String>,
    pub after: Option<DateTime<Utc>>,
    pub before: Option<DateTime<Utc>>,
    pub has_attachment: Option<bool>,
    pub unread: Option<bool>,
    pub important: Option<bool>,
    pub mailbox_id: Option<Uuid>,
    pub folder: Option<String>,
    pub label_id: Option<Uuid>,
    pub cursor: Option<String>,
}

#[derive(sqlx::FromRow)]
struct Hit {
    #[sqlx(flatten)]
    item: ItemRow,
    mailbox_address: String,
}

pub async fn search_messages(
    pool: &sqlx::PgPool,
    user: &CurrentUser,
    query: SearchQuery,
) -> Result<MessagePage, AppError> {
    if query.q.is_none()
        && query.from.is_none()
        && query.to.is_none()
        && query.subject.is_none()
        && query.after.is_none()
        && query.before.is_none()
        && query.has_attachment.is_none()
        && query.unread.is_none()
        && query.important.is_none()
        && query.folder.is_none()
        && query.label_id.is_none()
    {
        return Err(AppError::Validation("Enter a search.".into()));
    }
    if let Some(mailbox_id) = query.mailbox_id {
        require_reader(pool, mailbox_id, user)
            .await
            .map_err(hide_forbidden)?;
    }
    let (cursor_at, cursor_id) = super::listing::parse_cursor(query.cursor.as_deref())?;
    let rows = sqlx::query_as::<_, Hit>(
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
               m.has_attachments,
               mb.address::text AS mailbox_address
        FROM messages m
        JOIN mailboxes mb ON mb.id = m.mailbox_id
        JOIN mailbox_users mu
          ON mu.mailbox_id = m.mailbox_id
         AND mu.user_id = $1
         AND mu.can_read
        LEFT JOIN message_folder_assignments a ON a.message_id = m.id
        LEFT JOIN folders f ON f.id = a.folder_id
        WHERE ($2::uuid IS NULL OR m.mailbox_id = $2)
          AND ($3::text IS NULL OR m.search_vector @@ websearch_to_tsquery('english', $3))
          AND (
                $4::text IS NULL
                OR m.from_address::text ILIKE $4 ESCAPE '\'
                OR COALESCE(m.from_name, '') ILIKE $4 ESCAPE '\'
          )
          AND (
                $5::text IS NULL
                OR EXISTS (
                    SELECT 1 FROM recipients r
                    WHERE r.message_id = m.id
                      AND r.address::text ILIKE $5 ESCAPE '\'
                )
          )
          AND ($6::text IS NULL OR m.subject ILIKE $6 ESCAPE '\')
          AND ($7::timestamptz IS NULL OR COALESCE(m.received_at, m.sent_at, m.created_at) >= $7)
          AND ($8::timestamptz IS NULL OR COALESCE(m.received_at, m.sent_at, m.created_at) < $8)
          AND ($9::bool IS NULL OR m.has_attachments = $9)
          AND ($10::bool IS NULL OR m.is_read = NOT $10)
          AND ($11::bool IS NULL OR m.is_important = $11)
          AND ($12::uuid IS NULL OR EXISTS (
                SELECT 1 FROM message_labels ml
                WHERE ml.message_id = m.id AND ml.label_id = $12
          ))
          AND (
                $13::text IS NULL
                OR ($13 = 'starred' AND m.is_starred AND COALESCE(f.system_key, '') <> 'trash')
                OR ($13 = 'snoozed' AND m.snoozed_until > now() AND COALESCE(f.system_key, '') <> 'trash')
                OR (
                    $13 = 'inbox'
                    AND f.system_key = 'inbox'
                    AND (m.snoozed_until IS NULL OR m.snoozed_until <= now())
                )
                OR (
                    $13 NOT IN ('starred', 'snoozed', 'inbox')
                    AND (f.system_key = $13 OR f.id::text = $13)
                )
          )
          AND (
                $14::timestamptz IS NULL
                OR COALESCE(m.received_at, m.sent_at, m.created_at) < $14
                OR (
                    COALESCE(m.received_at, m.sent_at, m.created_at) = $14
                    AND m.id < $15
                )
          )
        ORDER BY COALESCE(m.received_at, m.sent_at, m.created_at) DESC, m.id DESC
        LIMIT $16
        "#,
    )
    .bind(user.id)
    .bind(query.mailbox_id)
    .bind(query.q.as_deref())
    .bind(query.from.as_deref())
    .bind(query.to.as_deref())
    .bind(query.subject.as_deref())
    .bind(query.after)
    .bind(query.before)
    .bind(query.has_attachment)
    .bind(query.unread)
    .bind(query.important)
    .bind(query.label_id)
    .bind(query.folder.as_deref())
    .bind(cursor_at)
    .bind(cursor_id)
    .bind(PAGE + 1)
    .fetch_all(pool)
    .await
    .map_err(search_error)?;
    let mut rows = rows;
    let next_cursor = if rows.len() > usize::try_from(PAGE).unwrap_or(50) {
        rows.pop();
        rows.last().map(|row| {
            format!(
                "{}|{}",
                row.item.activity_at.to_rfc3339(),
                row.item.id
            )
        })
    } else {
        None
    };
    let addresses: Vec<String> = rows.iter().map(|row| row.mailbox_address.clone()).collect();
    let items: Vec<ItemRow> = rows.into_iter().map(|row| row.item).collect();
    let mut messages = hydrate(pool, items).await?;
    for (message, address) in messages.iter_mut().zip(addresses) {
        message.mailbox_address = Some(address);
    }
    Ok(MessagePage {
        messages,
        next_cursor,
    })
}

pub fn parse_day(value: &str, end: bool) -> Result<DateTime<Utc>, AppError> {
    let day = NaiveDate::parse_from_str(value.trim(), "%Y-%m-%d")
        .map_err(|_| AppError::Validation("Use a date like 2026-11-16.".into()))?;
    let start = day
        .and_hms_opt(0, 0, 0)
        .ok_or_else(|| AppError::Validation("Use a date like 2026-11-16.".into()))?
        .and_utc();
    if end {
        Ok(start + chrono::Duration::days(1))
    } else {
        Ok(start)
    }
}

pub fn like_contains(value: &str) -> Result<String, AppError> {
    let value = value.trim();
    if value.is_empty() {
        return Err(AppError::Validation("Enter a search.".into()));
    }
    if value.chars().count() > 120 || value.chars().any(|character| character.is_control()) {
        return Err(AppError::Validation("That search is too long.".into()));
    }
    let escaped = value
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_");
    Ok(format!("%{escaped}%"))
}

pub fn plain_query(value: &str) -> Result<String, AppError> {
    let value = value.trim();
    if value.chars().count() > 200 || value.chars().any(|character| character.is_control()) {
        return Err(AppError::Validation("That search is too long.".into()));
    }
    if value.is_empty() {
        return Err(AppError::Validation("Enter a search.".into()));
    }
    Ok(value.to_string())
}

fn hide_forbidden(error: AppError) -> AppError {
    match error {
        AppError::Forbidden(_) => AppError::NotFound,
        other => other,
    }
}

fn search_error(error: sqlx::Error) -> AppError {
    let syntax = error.as_database_error().is_some_and(|db| {
        db.code().is_some_and(|code| code == "42601") || db.message().contains("syntax error")
    });
    if syntax {
        return AppError::Validation("That search could not be read.".into());
    }
    AppError::Database(error)
}
