//! Request budgets stored in PostgreSQL.
//!
//! The account lock still applies to a password. This limit covers the address
//! the request came from, so one source cannot spray every account.

use axum::http::HeaderMap;

use crate::api::AppError;
use sqlx::PgPool;

pub fn client_key(headers: &HeaderMap) -> String {
    let forwarded = headers
        .get("x-forwarded-for")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(',').next())
        .map(str::trim)
        .filter(|value| value.parse::<std::net::IpAddr>().is_ok());
    forwarded.unwrap_or("direct").to_string()
}

pub async fn charge(pool: &PgPool, bucket: &str, window_secs: i32, max: i32) -> Result<(), AppError> {
    let hits = sqlx::query_scalar::<_, i32>(
        r#"
        INSERT INTO rate_limits (bucket, hits, window_started_at)
        VALUES ($1, 1, now())
        ON CONFLICT (bucket) DO UPDATE
        SET hits = CASE
                WHEN rate_limits.window_started_at <= now() - ($2::int * interval '1 second')
                THEN 1
                ELSE rate_limits.hits + 1
            END,
            window_started_at = CASE
                WHEN rate_limits.window_started_at <= now() - ($2::int * interval '1 second')
                THEN now()
                ELSE rate_limits.window_started_at
            END
        RETURNING hits
        "#,
    )
    .bind(bucket)
    .bind(window_secs)
    .fetch_one(pool)
    .await?;
    if hits > max {
        return Err(AppError::Limited(
            "Too many requests. Try again shortly.".into(),
        ));
    }
    Ok(())
}
