use std::time::Duration;

use axum::body::Bytes;
use axum::http::HeaderMap;
use sqlx::FromRow;
use sqlx::PgPool;
use uuid::Uuid;

use crate::api::AppError;
use crate::providers::{EmailProvider, parse_webhook, unix_now, verify_signature};
use crate::state::AppState;

use super::ingest::{apply_delivery_status, delivery_status, store_inbound};
use super::provider_error;

const MAX_BODY: usize = 1_000_000;
const MAX_ATTEMPTS: i32 = 8;

pub async fn accept_webhook(
    state: &AppState,
    headers: &HeaderMap,
    body: &Bytes,
) -> Result<(), AppError> {
    let pool = crate::auth::pool_of(state)?;
    if body.len() > MAX_BODY {
        return Err(AppError::Validation("Webhook payload is too large.".into()));
    }
    crate::limit::charge(
        pool,
        &format!("webhook:{}", crate::limit::client_key(headers)),
        60,
        300,
    )
    .await?;
    let secret = state
        .config
        .resend
        .webhook_secret
        .as_deref()
        .ok_or_else(|| AppError::Unavailable("Webhook signing is not configured.".into()))?;
    let id = header(headers, "svix-id")?;
    let timestamp = header(headers, "svix-timestamp")?;
    let signature = header(headers, "svix-signature")?;
    if !verify_signature(secret, &id, &timestamp, &signature, body, unix_now()) {
        return Err(AppError::Forbidden(
            "Webhook signature was rejected.".into(),
        ));
    }
    let payload: serde_json::Value = serde_json::from_slice(body)
        .map_err(|_| AppError::Validation("Webhook payload is not valid.".into()))?;
    let notice = parse_webhook(&payload)
        .map_err(|_| AppError::Validation("Webhook payload is not a Resend event.".into()))?;
    sqlx::query(
        r#"
        INSERT INTO webhook_events (provider, provider_event_id, event_type, payload)
        VALUES ('resend', $1, $2, $3)
        ON CONFLICT (provider, provider_event_id) DO NOTHING
        "#,
    )
    .bind(&id)
    .bind(&notice.event_type)
    .bind(&payload)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn run_jobs(state: AppState) {
    let mut interval = tokio::time::interval(Duration::from_secs(2));
    loop {
        interval.tick().await;
        if let Err(error) = process_due(&state, 10).await {
            tracing::error!(%error, "provider queue failed");
        }
    }
}

pub async fn process_due(state: &AppState, limit: i32) -> Result<(), AppError> {
    let Some(pool) = &state.db else {
        return Ok(());
    };
    for _ in 0..limit {
        let Some(job) = claim(pool).await? else {
            return Ok(());
        };
        if let Err(error) = handle(state, &job).await {
            retry(pool, job.id, job.attempts, &error.to_string()).await?;
        }
    }
    Ok(())
}

#[derive(FromRow)]
struct Job {
    id: Uuid,
    payload: serde_json::Value,
    attempts: i32,
}

async fn claim(pool: &PgPool) -> Result<Option<Job>, AppError> {
    let mut tx = pool.begin().await?;
    let job = sqlx::query_as::<_, Job>(
        r#"
        WITH next AS (
            SELECT id
            FROM webhook_events
            WHERE attempts < 8
              AND (
                (status = 'pending' AND next_attempt_at <= now())
                OR (status = 'processing' AND updated_at < now() - interval '2 minutes')
              )
            ORDER BY next_attempt_at
            FOR UPDATE SKIP LOCKED
            LIMIT 1
        )
        UPDATE webhook_events AS event
        SET status = 'processing', attempts = event.attempts + 1
        FROM next
        WHERE event.id = next.id
        RETURNING event.id, event.payload, event.attempts
        "#,
    )
    .fetch_optional(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(job)
}

async fn handle(state: &AppState, job: &Job) -> Result<(), AppError> {
    let pool = crate::auth::pool_of(state)?;
    let notice = parse_webhook(&job.payload).map_err(provider_error)?;
    if notice.event_type == "email.received" {
        let email_id = notice
            .email_id
            .as_deref()
            .ok_or_else(|| AppError::Validation("Received mail is missing an email id.".into()))?;
        let client = state
            .provider
            .as_ref()
            .ok_or_else(|| AppError::Unavailable("Resend is not configured.".into()))?;
        let email = client
            .get_received_email(email_id)
            .await
            .map_err(provider_error)?;
        let stored = store_inbound(pool, &email, &notice.recipients).await?;
        for arrival in stored {
            if let Err(error) = crate::live::announce(state, &arrival).await {
                tracing::warn!(%error, "the inbox notice was not stored");
            }
        }
        if email.attachment_count > 0 {
            match client.list_received_attachments(email_id).await {
                Ok(files) => {
                    super::store_received_files(
                        pool,
                        &state.storage,
                        client,
                        state.config.scanner.as_deref(),
                        email_id,
                        &files,
                    )
                        .await?;
                }
                Err(crate::providers::ProviderError::Rejected { .. })
                | Err(crate::providers::ProviderError::Unexpected) => {
                    tracing::info!(
                        email_id,
                        attachments = email.attachment_count,
                        "received attachment metadata without stored bytes"
                    );
                }
                Err(error) => return Err(provider_error(error)),
            }
        }
    } else if let (Some(status), Some(email_id)) = (
        delivery_status(&notice.event_type),
        notice.email_id.as_deref(),
    ) {
        apply_delivery_status(pool, email_id, status).await?;
    }
    sqlx::query(
        r#"
        UPDATE webhook_events
        SET status = 'processed', processed_at = now(), last_error = NULL
        WHERE id = $1
        "#,
    )
    .bind(job.id)
    .execute(pool)
    .await?;
    Ok(())
}

async fn retry(pool: &PgPool, id: Uuid, attempts: i32, message: &str) -> Result<(), AppError> {
    let delay = (15_i64 * 2_i64.saturating_pow((attempts.max(1) - 1) as u32)).min(300);
    let failed = attempts >= MAX_ATTEMPTS;
    sqlx::query(
        r#"
        UPDATE webhook_events
        SET status = CASE WHEN $2 THEN 'failed' ELSE 'pending' END,
            next_attempt_at = now() + ($3 * interval '1 second'),
            last_error = $4
        WHERE id = $1
        "#,
    )
    .bind(id)
    .bind(failed)
    .bind(delay)
    .bind(message)
    .execute(pool)
    .await?;
    if failed {
        tracing::error!(%id, "provider event failed");
    } else {
        tracing::warn!(%id, delay_seconds = delay, "provider event will retry");
    }
    Ok(())
}

fn header(headers: &HeaderMap, name: &str) -> Result<String, AppError> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .ok_or_else(|| AppError::Forbidden("Webhook signature was rejected.".into()))
}
