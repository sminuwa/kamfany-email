//! Live notices for people who can read a mailbox.
//!
//! A notification row is durable. The socket only reaches browsers connected
//! to this API process. A refresh still shows the stored notice.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use axum::extract::ws::{Message, WebSocket};
use futures_util::{SinkExt, StreamExt};
use serde::Serialize;
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::api::AppError;
use crate::auth::pool_of;
use crate::state::AppState;

#[derive(Clone, Debug)]
pub struct LiveHub {
    inner: Arc<Mutex<HashMap<Uuid, Vec<mpsc::UnboundedSender<String>>>>>,
}

impl LiveHub {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn subscribe(&self, user_id: Uuid) -> mpsc::UnboundedReceiver<String> {
        let (sender, receiver) = mpsc::unbounded_channel();
        self.lock().entry(user_id).or_default().push(sender);
        receiver
    }

    pub fn publish(&self, user_id: Uuid, payload: &str) {
        let mut readers = self.lock();
        let Some(channels) = readers.get_mut(&user_id) else {
            return;
        };
        channels.retain(|channel| channel.send(payload.to_string()).is_ok());
        if channels.is_empty() {
            readers.remove(&user_id);
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<Uuid, Vec<mpsc::UnboundedSender<String>>>> {
        self.inner.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

impl Default for LiveHub {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct Arrival {
    pub mailbox_id: Uuid,
    pub message_id: Uuid,
    pub from_name: Option<String>,
    pub from_address: String,
    pub subject: String,
}

#[derive(Debug, sqlx::FromRow)]
struct NoticeRow {
    id: Uuid,
    user_id: Uuid,
}

#[derive(Serialize)]
struct LiveEvent<'a> {
    #[serde(rename = "type")]
    kind: &'a str,
    notification_id: Uuid,
    mailbox_id: Uuid,
    message_id: Uuid,
    title: &'a str,
    body: &'a str,
    inbox_unread: i64,
}

pub async fn announce(state: &AppState, arrival: &Arrival) -> Result<(), AppError> {
    let pool = pool_of(state)?;
    let title = arrival
        .from_name
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(arrival.from_address.as_str());
    let body = if arrival.subject.trim().is_empty() {
        "(no subject)"
    } else {
        arrival.subject.trim()
    };
    let rows = sqlx::query_as::<_, NoticeRow>(
        r#"
        INSERT INTO notifications (user_id, mailbox_id, message_id, kind, title, body, payload)
        SELECT mu.user_id, $1, $2, 'mail.received', $3, $4,
               jsonb_build_object('message_id', $2, 'mailbox_id', $1)
        FROM mailbox_users mu
        WHERE mu.mailbox_id = $1 AND mu.can_read
        ON CONFLICT (user_id, message_id) WHERE message_id IS NOT NULL DO NOTHING
        RETURNING id, user_id
        "#,
    )
    .bind(arrival.mailbox_id)
    .bind(arrival.message_id)
    .bind(title)
    .bind(body)
    .fetch_all(pool)
    .await?;
    let readers = rows.len();
    if readers == 0 {
        return Ok(());
    }
    let inbox_unread = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT count(*)
        FROM message_folder_assignments a
        JOIN messages m ON m.id = a.message_id
        JOIN folders f ON f.id = a.folder_id
        WHERE m.mailbox_id = $1
          AND f.system_key = 'inbox'
          AND NOT m.is_read
          AND (m.snoozed_until IS NULL OR m.snoozed_until <= now())
        "#,
    )
    .bind(arrival.mailbox_id)
    .fetch_one(pool)
    .await?;
    for row in rows {
        let payload = serde_json::to_string(&LiveEvent {
            kind: "mail.received",
            notification_id: row.id,
            mailbox_id: arrival.mailbox_id,
            message_id: arrival.message_id,
            title,
            body,
            inbox_unread,
        })
        .map_err(|_| AppError::Internal)?;
        state.live.publish(row.user_id, &payload);
    }
    tracing::info!(
        mailbox_id = %arrival.mailbox_id,
        message_id = %arrival.message_id,
        readers,
        "notified mailbox readers"
    );
    Ok(())
}

pub async fn drive(mut socket: WebSocket, mut notices: mpsc::UnboundedReceiver<String>) {
    let hello = Message::text(r#"{"type":"ready"}"#);
    if socket.send(hello).await.is_err() {
        return;
    }
    let (mut outgoing, mut incoming) = socket.split();
    loop {
        tokio::select! {
            incoming = incoming.next() => {
                match incoming {
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Ok(Message::Ping(payload))) => {
                        if outgoing.send(Message::Pong(payload)).await.is_err() {
                            break;
                        }
                    }
                    Some(Err(_)) => break,
                    _ => {}
                }
            }
            notice = notices.recv() => {
                let Some(text) = notice else { break };
                if outgoing.send(Message::text(text)).await.is_err() {
                    break;
                }
            }
        }
    }
}

pub async fn list_notices(
    state: &AppState,
    user_id: Uuid,
) -> Result<Vec<Notice>, AppError> {
    let pool = pool_of(state)?;
    let rows = sqlx::query_as::<_, Notice>(
        r#"
        SELECT id, mailbox_id, message_id, kind, title, body, read_at, created_at
        FROM notifications
        WHERE user_id = $1
        ORDER BY created_at DESC
        LIMIT 30
        "#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn mark_notice(
    state: &AppState,
    user_id: Uuid,
    notice_id: Uuid,
) -> Result<(), AppError> {
    let pool = pool_of(state)?;
    let updated = sqlx::query(
        "UPDATE notifications SET read_at = COALESCE(read_at, now()) WHERE id = $1 AND user_id = $2",
    )
    .bind(notice_id)
    .bind(user_id)
    .execute(pool)
    .await?;
    if updated.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    Ok(())
}

pub async fn mark_all(state: &AppState, user_id: Uuid) -> Result<(), AppError> {
    let pool = pool_of(state)?;
    sqlx::query(
        "UPDATE notifications SET read_at = now() WHERE user_id = $1 AND read_at IS NULL",
    )
    .bind(user_id)
    .execute(pool)
    .await?;
    Ok(())
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct Notice {
    pub id: Uuid,
    pub mailbox_id: Option<Uuid>,
    pub message_id: Option<Uuid>,
    pub kind: String,
    pub title: String,
    pub body: String,
    pub read_at: Option<chrono::DateTime<chrono::Utc>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}
