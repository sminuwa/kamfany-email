use serde::Serialize;

use crate::api::AppError;
use crate::auth::pool_of;
use crate::providers::{EmailProvider, ProviderError};
use crate::state::AppState;

use super::provider_error;

#[derive(Debug, Serialize)]
pub struct ProviderReport {
    pub configured: bool,
    pub webhook_configured: bool,
    pub sending_domain: String,
    pub domain_status: String,
    pub receiving: String,
    pub verified_addresses: Vec<String>,
    pub scanner: String,
    pub queue_received: i64,
    pub queue_processing: i64,
    pub queue_failed: i64,
    pub queue_oldest_seconds: Option<i64>,
}

pub async fn provider_report(state: &AppState) -> Result<ProviderReport, AppError> {
    report(
        state,
        state.config.resend.api_key.is_some(),
        "not_checked".to_string(),
        "unknown".to_string(),
    )
    .await
}

pub async fn sync_provider(state: &AppState) -> Result<ProviderReport, AppError> {
    let pool = pool_of(state)?;
    let client = state
        .provider
        .as_ref()
        .ok_or_else(|| AppError::Unavailable("Resend is not configured.".into()))?;
    let domains = client.list_domains().await.map_err(provider_error)?;
    let domain = domains.iter().find(|item| {
        item.name
            .eq_ignore_ascii_case(&state.config.resend.sending_domain)
    });
    let (domain_status, receiving) = match domain {
        Some(domain) => (
            domain.status.clone(),
            domain
                .receiving
                .clone()
                .unwrap_or_else(|| "unknown".to_string()),
        ),
        None => ("missing".to_string(), "unknown".to_string()),
    };

    match client.list_inboxes().await {
        Ok(inboxes) => {
            for inbox in inboxes {
                sqlx::query(
                    r#"
                    UPDATE mailboxes
                    SET provider_inbox_id = $1,
                        is_receiving_verified = true,
                        verified_at = COALESCE(verified_at, now())
                    WHERE provider = 'resend'
                      AND lower(address::text) = lower($2)
                    "#,
                )
                .bind(&inbox.id)
                .bind(&inbox.address)
                .execute(pool)
                .await?;
            }
        }
        Err(ProviderError::Rejected { status: 404, .. }) => {
            tracing::info!("resend inboxes are not available on this account");
        }
        Err(error) => return Err(provider_error(error)),
    }

    report(state, true, domain_status, receiving).await
}

async fn report(
    state: &AppState,
    configured: bool,
    domain_status: String,
    receiving: String,
) -> Result<ProviderReport, AppError> {
    let queue = queue_counts(state).await?;
    Ok(ProviderReport {
        configured,
        webhook_configured: state.config.resend.webhook_secret.is_some(),
        sending_domain: state.config.resend.sending_domain.clone(),
        domain_status,
        receiving,
        verified_addresses: verified(state).await?,
        scanner: if state.config.scanner.is_some() {
            "clamav".to_string()
        } else {
            "not_configured".to_string()
        },
        queue_received: queue.received,
        queue_processing: queue.processing,
        queue_failed: queue.failed,
        queue_oldest_seconds: queue.oldest_unprocessed_seconds,
    })
}

#[derive(sqlx::FromRow)]
struct QueueCounts {
    received: i64,
    processing: i64,
    failed: i64,
    oldest_unprocessed_seconds: Option<i64>,
}

async fn queue_counts(state: &AppState) -> Result<QueueCounts, AppError> {
    let Some(pool) = &state.db else {
        return Ok(QueueCounts {
            received: 0,
            processing: 0,
            failed: 0,
            oldest_unprocessed_seconds: None,
        });
    };
    let row = sqlx::query_as::<_, QueueCounts>(
        r#"
        SELECT
            count(*) FILTER (WHERE status = 'pending') AS received,
            count(*) FILTER (WHERE status = 'processing') AS processing,
            count(*) FILTER (WHERE status = 'failed') AS failed,
            EXTRACT(
                EPOCH FROM (
                    now() - min(created_at) FILTER (WHERE status IN ('pending', 'processing'))
                )
            )::bigint AS oldest_unprocessed_seconds
        FROM webhook_events
        "#,
    )
    .fetch_one(pool)
    .await?;
    Ok(row)
}

async fn verified(state: &AppState) -> Result<Vec<String>, AppError> {
    let Some(pool) = &state.db else {
        return Ok(Vec::new());
    };
    let rows = sqlx::query_scalar::<_, String>(
        r#"
        SELECT address::text
        FROM mailboxes
        WHERE is_receiving_verified
        ORDER BY address
        "#,
    )
    .fetch_all(pool)
    .await?;
    Ok(rows)
}
