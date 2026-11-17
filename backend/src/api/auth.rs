use std::sync::OnceLock;

use axum::Json;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, HeaderValue, header};
use axum::response::{IntoResponse, Response};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use crate::api::body::Body;
use crate::api::errors::AppError;
use crate::api::response::ApiData;
use crate::auth::{
    CurrentUser, Profile, clear_session_cookie, hash_password, hash_token, load_profile, new_token,
    normalize_email, password_policy, pool_of, session_cookie, session_expiry, verify_password,
    write_audit,
};
use crate::providers::{EmailProvider, OutboundEmail};
use crate::state::AppState;

const LOCK_AFTER: i32 = 5;
const RESET_MINUTES: i64 = 30;
const RESET_LATER: &str =
    "If that account exists, a reset will be sent when mail delivery is connected.";
const RESET_SENT: &str = "If that account can receive mail, a reset link is on its way.";

#[derive(Deserialize)]
pub struct LoginBody {
    email: String,
    password: String,
}

#[derive(Deserialize)]
pub struct PasswordBody {
    current_password: String,
    new_password: String,
}

#[derive(Deserialize)]
pub struct NameBody {
    display_name: String,
}

#[derive(Deserialize)]
pub struct ForgotBody {
    email: String,
}

#[derive(Deserialize)]
pub struct ResetBody {
    token: String,
    password: String,
}

#[derive(Serialize, FromRow)]
pub struct SessionItem {
    pub id: Uuid,
    pub user_agent: Option<String>,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub current: bool,
}

#[derive(Serialize)]
pub struct Notice {
    message: &'static str,
}

#[derive(FromRow)]
struct UserSecret {
    id: Uuid,
    password_hash: String,
    is_active: bool,
    failed_login_count: i32,
    locked_until: Option<DateTime<Utc>>,
}

pub async fn login(
    State(state): State<AppState>,
    headers: HeaderMap,
    Body(body): Body<LoginBody>,
) -> Result<Response, AppError> {
    let pool = pool_of(&state)?;
    crate::limit::charge(
        pool,
        &format!("login:{}", crate::limit::client_key(&headers)),
        900,
        120,
    )
    .await?;
    let email = normalize_email(&body.email)?;
    let user = sqlx::query_as::<_, UserSecret>(
        r#"
        SELECT id, password_hash, is_active, failed_login_count, locked_until
        FROM users
        WHERE email = $1
        "#,
    )
    .bind(&email)
    .fetch_optional(pool)
    .await?;

    let (hash, active, locked) = match &user {
        Some(row) => (
            row.password_hash.as_str(),
            row.is_active,
            row.locked_until.is_some_and(|until| until > Utc::now()),
        ),
        None => (dummy_hash(), false, false),
    };
    let password_ok = verify_password(&body.password, hash);
    let Some(user) = user else {
        tracing::info!("login failed for an unknown account");
        return Err(AppError::InvalidCredentials);
    };
    if !active || locked || !password_ok {
        if active && !locked && !password_ok {
            record_failure(pool, user.id, user.failed_login_count).await?;
        }
        write_audit(
            pool,
            Some(user.id),
            "auth.login_failed",
            Some("user"),
            Some(user.id),
            serde_json::json!({}),
        )
        .await?;
        tracing::info!(user_id = %user.id, "login failed");
        return Err(AppError::InvalidCredentials);
    }

    sqlx::query(
        r#"
        UPDATE users
        SET failed_login_count = 0, locked_until = NULL, last_login_at = now()
        WHERE id = $1
        "#,
    )
    .bind(user.id)
    .execute(pool)
    .await?;

    let token = new_token();
    let agent = client_agent(&headers);
    sqlx::query(
        r#"
        INSERT INTO sessions (user_id, token_hash, user_agent, expires_at)
        VALUES ($1, $2, $3, $4)
        "#,
    )
    .bind(user.id)
    .bind(hash_token(&token))
    .bind(&agent)
    .bind(session_expiry())
    .execute(pool)
    .await?;
    write_audit(
        pool,
        Some(user.id),
        "auth.login",
        Some("user"),
        Some(user.id),
        serde_json::json!({}),
    )
    .await?;
    let profile = load_profile(pool, user.id).await?;
    Ok(with_session(&state, &token, profile))
}

pub async fn logout(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Response, AppError> {
    let pool = pool_of(&state)?;
    if let Some(token) = crate::auth::read_cookie(&headers, crate::auth::SESSION_COOKIE) {
        let hash = hash_token(&token);
        let user_id = sqlx::query_scalar::<_, Uuid>(
            "SELECT user_id FROM sessions WHERE token_hash = $1 AND revoked_at IS NULL",
        )
        .bind(&hash)
        .fetch_optional(pool)
        .await?;
        sqlx::query("UPDATE sessions SET revoked_at = now() WHERE token_hash = $1")
            .bind(&hash)
            .execute(pool)
            .await?;
        if let Some(user_id) = user_id {
            write_audit(
                pool,
                Some(user_id),
                "auth.logout",
                Some("user"),
                Some(user_id),
                serde_json::json!({}),
            )
            .await?;
        }
    }
    Ok(cleared(&state))
}

pub async fn refresh(
    State(state): State<AppState>,
    user: CurrentUser,
    headers: HeaderMap,
) -> Result<Response, AppError> {
    let pool = pool_of(&state)?;
    let token = new_token();
    sqlx::query(
        r#"
        UPDATE sessions
        SET token_hash = $2, expires_at = $3, user_agent = $4
        WHERE id = $1 AND revoked_at IS NULL
        "#,
    )
    .bind(user.session_id)
    .bind(hash_token(&token))
    .bind(session_expiry())
    .bind(client_agent(&headers))
    .execute(pool)
    .await?;
    let profile = load_profile(pool, user.id).await?;
    Ok(with_session(&state, &token, profile))
}

pub async fn me(
    State(state): State<AppState>,
    user: CurrentUser,
) -> Result<Json<ApiData<Profile>>, AppError> {
    let profile = load_profile(pool_of(&state)?, user.id).await?;
    Ok(Json(ApiData { data: profile }))
}

pub async fn update_me(
    State(state): State<AppState>,
    user: CurrentUser,
    Body(body): Body<NameBody>,
) -> Result<Json<ApiData<Profile>>, AppError> {
    let pool = pool_of(&state)?;
    let name = crate::auth::display_name(&body.display_name)?;
    sqlx::query("UPDATE users SET display_name = $2 WHERE id = $1")
        .bind(user.id)
        .bind(&name)
        .execute(pool)
        .await?;
    let profile = load_profile(pool, user.id).await?;
    Ok(Json(ApiData { data: profile }))
}

pub async fn change_password(
    State(state): State<AppState>,
    user: CurrentUser,
    Body(body): Body<PasswordBody>,
) -> Result<Json<ApiData<Notice>>, AppError> {
    let pool = pool_of(&state)?;
    password_policy(&body.new_password)?;
    if body.current_password == body.new_password {
        return Err(AppError::Validation("Choose a different password.".into()));
    }
    let hash: String = sqlx::query_scalar("SELECT password_hash FROM users WHERE id = $1")
        .bind(user.id)
        .fetch_one(pool)
        .await?;
    if !verify_password(&body.current_password, &hash) {
        return Err(AppError::Validation(
            "The current password is incorrect.".into(),
        ));
    }
    let next = hash_password(&body.new_password)?;
    sqlx::query("UPDATE users SET password_hash = $2 WHERE id = $1")
        .bind(user.id)
        .bind(&next)
        .execute(pool)
        .await?;
    sqlx::query(
        r#"
        UPDATE sessions
        SET revoked_at = now()
        WHERE user_id = $1 AND id <> $2 AND revoked_at IS NULL
        "#,
    )
    .bind(user.id)
    .bind(user.session_id)
    .execute(pool)
    .await?;
    write_audit(
        pool,
        Some(user.id),
        "auth.password_changed",
        Some("user"),
        Some(user.id),
        serde_json::json!({}),
    )
    .await?;
    Ok(Json(ApiData {
        data: Notice {
            message: "Password updated. Other sessions were signed out.",
        },
    }))
}

pub async fn sessions(
    State(state): State<AppState>,
    user: CurrentUser,
) -> Result<Json<ApiData<Vec<SessionItem>>>, AppError> {
    let rows = sqlx::query_as::<_, SessionItem>(
        r#"
        SELECT id, user_agent, created_at, expires_at, (id = $2) AS current
        FROM sessions
        WHERE user_id = $1 AND revoked_at IS NULL AND expires_at > now()
        ORDER BY created_at DESC
        "#,
    )
    .bind(user.id)
    .bind(user.session_id)
    .fetch_all(pool_of(&state)?)
    .await?;
    Ok(Json(ApiData { data: rows }))
}

pub async fn revoke_session(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(session_id): Path<Uuid>,
) -> Result<Response, AppError> {
    let pool = pool_of(&state)?;
    let result = sqlx::query(
        r#"
        UPDATE sessions
        SET revoked_at = now()
        WHERE id = $1 AND user_id = $2 AND revoked_at IS NULL
        "#,
    )
    .bind(session_id)
    .bind(user.id)
    .execute(pool)
    .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    write_audit(
        pool,
        Some(user.id),
        "auth.session_revoked",
        Some("session"),
        Some(session_id),
        serde_json::json!({}),
    )
    .await?;
    if session_id == user.session_id {
        return Ok(cleared(&state));
    }
    Ok(Json(ApiData {
        data: Notice {
            message: "Session signed out.",
        },
    })
    .into_response())
}

pub async fn forgot_password(
    State(state): State<AppState>,
    headers: HeaderMap,
    Body(body): Body<ForgotBody>,
) -> Result<Json<ApiData<Notice>>, AppError> {
    if let Ok(pool) = pool_of(&state) {
        crate::limit::charge(
            pool,
            &format!("reset:{}", crate::limit::client_key(&headers)),
            3600,
            10,
        )
        .await?;
    }
    let email = normalize_email(&body.email)?;
    let (Some(pool), Some(provider), Some(origin)) = (
        pool_of(&state).ok(),
        state.provider.as_ref(),
        state
            .config
            .brand
            .public_url
            .as_deref()
            .or_else(|| public_origin(&state.config.cors_origins)),
    ) else {
        return Ok(notice(RESET_LATER));
    };
    let user_id =
        sqlx::query_scalar::<_, Uuid>("SELECT id FROM users WHERE email = $1 AND is_active")
            .bind(&email)
            .fetch_optional(pool)
            .await?;
    if let Some(user_id) = user_id {
        let token = new_token();
        let token_hash = hash_token(&token);
        let mut tx = pool.begin().await?;
        sqlx::query(
            r#"
            UPDATE password_resets
            SET used_at = now()
            WHERE user_id = $1 AND used_at IS NULL
            "#,
        )
        .bind(user_id)
        .execute(&mut *tx)
        .await?;
        let reset_id = sqlx::query_scalar::<_, Uuid>(
            r#"
            INSERT INTO password_resets (user_id, token_hash, expires_at)
            VALUES ($1, $2, now() + ($3::int * interval '1 minute'))
            RETURNING id
            "#,
        )
        .bind(user_id)
        .bind(&token_hash)
        .bind(i32::try_from(RESET_MINUTES).unwrap_or(30))
        .fetch_one(&mut *tx)
        .await?;
        tx.commit().await?;
        let link = format!("{origin}/reset-password?token={token}");
        if let Err(error) = provider
            .send_email(&reset_mail(
                &state.config.brand.name,
                &state.config.resend.sending_domain,
                &email,
                &link,
                reset_id,
            ))
            .await
        {
            tracing::warn!(%error, "password reset mail was not accepted");
            sqlx::query("UPDATE password_resets SET used_at = now() WHERE id = $1")
                .bind(reset_id)
                .execute(pool)
                .await?;
        }
    }
    Ok(notice(RESET_SENT))
}

pub async fn reset_password(
    State(state): State<AppState>,
    Body(body): Body<ResetBody>,
) -> Result<Json<ApiData<Notice>>, AppError> {
    let pool = pool_of(&state)?;
    password_policy(&body.password)?;
    let token = body.token.trim().to_ascii_lowercase();
    if token.len() != 64 || !token.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(AppError::Validation("This reset link is not valid.".into()));
    }
    let row = sqlx::query_as::<_, ResetRow>(
        r#"
        SELECT id, user_id
        FROM password_resets
        WHERE token_hash = $1 AND used_at IS NULL AND expires_at > now()
        "#,
    )
    .bind(hash_token(&token))
    .fetch_optional(pool)
    .await?;
    let Some(row) = row else {
        return Err(AppError::Validation("This reset link is not valid.".into()));
    };
    let next = hash_password(&body.password)?;
    let mut tx = pool.begin().await?;
    let used = sqlx::query(
        r#"
        UPDATE password_resets
        SET used_at = now()
        WHERE id = $1 AND used_at IS NULL AND expires_at > now()
        "#,
    )
    .bind(row.id)
    .execute(&mut *tx)
    .await?;
    if used.rows_affected() != 1 {
        return Err(AppError::Validation("This reset link is not valid.".into()));
    }
    sqlx::query(
        r#"
        UPDATE users
        SET password_hash = $2, failed_login_count = 0, locked_until = NULL
        WHERE id = $1
        "#,
    )
    .bind(row.user_id)
    .bind(&next)
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        r#"
        UPDATE sessions
        SET revoked_at = now()
        WHERE user_id = $1 AND revoked_at IS NULL
        "#,
    )
    .bind(row.user_id)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    write_audit(
        pool,
        Some(row.user_id),
        "auth.password_reset",
        Some("user"),
        Some(row.user_id),
        serde_json::json!({}),
    )
    .await?;
    Ok(notice("Password updated. Sign in with the new password."))
}

fn notice(message: &'static str) -> Json<ApiData<Notice>> {
    Json(ApiData {
        data: Notice { message },
    })
}

fn public_origin(origins: &[String]) -> Option<&str> {
    origins
        .iter()
        .find(|origin| origin.starts_with("https://"))
        .or_else(|| origins.first())
        .map(String::as_str)
}

fn reset_mail(brand: &str, domain: &str, email: &str, link: &str, reset_id: Uuid) -> OutboundEmail {
    let text = format!(
        "A password reset was requested for {email}.\n\nOpen this link within {RESET_MINUTES} minutes:\n{link}\n\nIf you did not ask for this, ignore the message. The password stays the same.\n"
    );
    OutboundEmail {
        from_name: brand.to_string(),
        from_address: format!("noreply@{domain}"),
        to: vec![email.to_string()],
        cc: Vec::new(),
        bcc: Vec::new(),
        subject: format!("Reset your {brand} password"),
        text: Some(text),
        html: None,
        idempotency_key: reset_id.to_string(),
        headers: Vec::new(),
        attachments: Vec::new(),
    }
}

#[derive(FromRow)]
struct ResetRow {
    id: Uuid,
    user_id: Uuid,
}

async fn record_failure(pool: &sqlx::PgPool, user_id: Uuid, current: i32) -> Result<(), AppError> {
    let lock = current + 1 >= LOCK_AFTER;
    sqlx::query(
        r#"
        UPDATE users
        SET failed_login_count = $2, locked_until = $3
        WHERE id = $1
        "#,
    )
    .bind(user_id)
    .bind(if lock { 0 } else { current + 1 })
    .bind(lock.then(|| Utc::now() + Duration::minutes(15)))
    .execute(pool)
    .await?;
    Ok(())
}

fn with_session(state: &AppState, token: &str, profile: Profile) -> Response {
    (
        [
            (header::SET_COOKIE, session_cookie(token, state.config.env)),
            (header::CACHE_CONTROL, HeaderValue::from_static("no-store")),
        ],
        Json(ApiData { data: profile }),
    )
        .into_response()
}

fn cleared(state: &AppState) -> Response {
    (
        [
            (header::SET_COOKIE, clear_session_cookie(state.config.env)),
            (header::CACHE_CONTROL, HeaderValue::from_static("no-store")),
        ],
        Json(ApiData {
            data: Notice {
                message: "Signed out.",
            },
        }),
    )
        .into_response()
}

fn client_agent(headers: &HeaderMap) -> Option<String> {
    let value = headers
        .get(header::USER_AGENT)
        .and_then(|value| value.to_str().ok())?
        .trim();
    if value.is_empty() {
        None
    } else {
        Some(value.chars().take(300).collect())
    }
}

fn dummy_hash() -> &'static str {
    static DUMMY: OnceLock<String> = OnceLock::new();
    DUMMY.get_or_init(|| hash_password("not-a-real-account-password").expect("dummy password hash"))
}
