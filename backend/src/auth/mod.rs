mod password;
mod token;

pub use password::{
    display_name, hash_password, normalize_email, password_policy, verify_password,
};
pub use token::{
    SESSION_COOKIE, browser_request_allowed, clear_session_cookie, hash_token, new_token,
    read_cookie, session_cookie,
};

use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::api::AppError;
use crate::state::AppState;

#[derive(Debug, Clone)]
pub struct CurrentUser {
    pub id: Uuid,
    pub session_id: Uuid,
    pub email: String,
    pub display_name: String,
    pub permissions: Vec<String>,
}

impl CurrentUser {
    pub fn allows(&self, key: &str) -> bool {
        self.permissions.iter().any(|item| item == key)
    }
}

#[derive(Debug, Serialize)]
pub struct Profile {
    pub id: Uuid,
    pub email: String,
    pub display_name: String,
    pub roles: Vec<String>,
    pub permissions: Vec<String>,
}

#[derive(sqlx::FromRow)]
struct SessionUser {
    id: Uuid,
    session_id: Uuid,
    email: String,
    display_name: String,
    is_active: bool,
}

pub async fn user_from_token(
    pool: &PgPool,
    token_hash: &str,
) -> Result<Option<CurrentUser>, AppError> {
    let row = sqlx::query_as::<_, SessionUser>(
        r#"
        SELECT u.id,
               s.id AS session_id,
               u.email::text AS email,
               u.display_name,
               u.is_active
        FROM sessions s
        JOIN users u ON u.id = s.user_id
        WHERE s.token_hash = $1
          AND s.revoked_at IS NULL
          AND s.expires_at > now()
        "#,
    )
    .bind(token_hash)
    .fetch_optional(pool)
    .await?;

    let Some(row) = row else {
        return Ok(None);
    };
    if !row.is_active {
        return Ok(None);
    }
    let permissions = load_permissions(pool, row.id).await?;
    Ok(Some(CurrentUser {
        id: row.id,
        session_id: row.session_id,
        email: row.email,
        display_name: row.display_name,
        permissions,
    }))
}

pub async fn load_profile(pool: &PgPool, user_id: Uuid) -> Result<Profile, AppError> {
    let row = sqlx::query_as::<_, (Uuid, String, String, bool)>(
        r#"
        SELECT id, email::text, display_name, is_active
        FROM users
        WHERE id = $1
        "#,
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await?;
    let Some((id, email, display_name, is_active)) = row else {
        return Err(AppError::Unauthorized);
    };
    if !is_active {
        return Err(AppError::Unauthorized);
    }
    let roles = load_roles(pool, id).await?;
    let permissions = load_permissions(pool, id).await?;
    Ok(Profile {
        id,
        email,
        display_name,
        roles,
        permissions,
    })
}

pub async fn load_roles(pool: &PgPool, user_id: Uuid) -> Result<Vec<String>, AppError> {
    let rows = sqlx::query_scalar::<_, String>(
        r#"
        SELECT r.name
        FROM user_roles ur
        JOIN roles r ON r.id = ur.role_id
        WHERE ur.user_id = $1
        ORDER BY r.name
        "#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn load_permissions(pool: &PgPool, user_id: Uuid) -> Result<Vec<String>, AppError> {
    let rows = sqlx::query_scalar::<_, String>(
        r#"
        SELECT DISTINCT p.key
        FROM user_roles ur
        JOIN role_permissions rp ON rp.role_id = ur.role_id
        JOIN permissions p ON p.id = rp.permission_id
        WHERE ur.user_id = $1
        ORDER BY p.key
        "#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn write_audit(
    pool: &PgPool,
    actor: Option<Uuid>,
    action: &str,
    entity_type: Option<&str>,
    entity_id: Option<Uuid>,
    metadata: serde_json::Value,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        INSERT INTO audit_logs (actor_user_id, action, entity_type, entity_id, metadata)
        VALUES ($1, $2, $3, $4, $5)
        "#,
    )
    .bind(actor)
    .bind(action)
    .bind(entity_type)
    .bind(entity_id)
    .bind(metadata)
    .execute(pool)
    .await?;
    Ok(())
}

pub fn pool_of(state: &AppState) -> Result<&PgPool, AppError> {
    state.db.as_ref().ok_or(AppError::NotReady)
}

impl FromRequestParts<AppState> for CurrentUser {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let Some(token) = read_cookie(&parts.headers, SESSION_COOKIE) else {
            return Err(AppError::Unauthorized);
        };
        let Some(user) = user_from_token(pool_of(state)?, &hash_token(&token)).await? else {
            return Err(AppError::Unauthorized);
        };
        Ok(user)
    }
}

pub fn session_expiry() -> DateTime<Utc> {
    Utc::now() + chrono::Duration::days(7)
}

pub fn unique_or(error: sqlx::Error, conflict: &str) -> AppError {
    if error
        .as_database_error()
        .is_some_and(|db| db.is_unique_violation())
    {
        AppError::Conflict(conflict.to_string())
    } else {
        AppError::Database(error)
    }
}
