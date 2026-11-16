use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::Response;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use crate::api::body::Body;
use crate::api::errors::AppError;
use crate::api::response::ApiData;
use crate::auth::{CurrentUser, display_name, pool_of, write_audit};
use crate::state::AppState;

#[derive(Debug, Serialize, FromRow)]
pub struct MailboxView {
    pub id: Uuid,
    pub address: String,
    pub display_name: String,
    pub is_enabled: bool,
    pub is_receiving_verified: bool,
    pub can_read: bool,
    pub can_send: bool,
    pub can_manage: bool,
    pub is_default: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, FromRow)]
pub struct MemberView {
    pub id: Uuid,
    pub email: String,
    pub display_name: String,
    pub can_read: bool,
    pub can_send: bool,
    pub can_manage: bool,
    pub is_default: bool,
}

#[derive(Deserialize)]
#[allow(dead_code)]
pub struct MailboxBody {
    address: String,
    display_name: String,
}

#[derive(Deserialize)]
pub struct MailboxPatch {
    display_name: Option<String>,
    is_enabled: Option<bool>,
}

#[derive(Deserialize)]
#[allow(dead_code)]
pub struct GrantBody {
    email: String,
    can_read: Option<bool>,
    can_send: Option<bool>,
    can_manage: Option<bool>,
}

#[derive(FromRow)]
struct MailboxRow {
    id: Uuid,
    address: String,
    display_name: String,
    is_enabled: bool,
    is_receiving_verified: bool,
    can_read: Option<bool>,
    can_send: Option<bool>,
    can_manage: Option<bool>,
    is_default: Option<bool>,
    created_at: DateTime<Utc>,
}

pub async fn list_mailboxes(
    State(state): State<AppState>,
    user: CurrentUser,
) -> Result<Json<ApiData<Vec<MailboxView>>>, AppError> {
    let pool = pool_of(&state)?;
    let platform = user.allows("mailbox.manage");
    let rows = sqlx::query_as::<_, MailboxRow>(MAILBOX_LIST)
        .bind(user.id)
        .fetch_all(pool)
        .await?;
    Ok(Json(ApiData {
        data: rows
            .into_iter()
            .map(|row| view_from(row, platform))
            .collect(),
    }))
}

pub async fn get_mailbox(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiData<MailboxView>>, AppError> {
    let view = load_mailbox(pool_of(&state)?, id, &user).await?;
    Ok(Json(ApiData { data: view }))
}

pub async fn create_mailbox(
    State(state): State<AppState>,
    user: CurrentUser,
    Body(body): Body<MailboxBody>,
) -> Result<Response, AppError> {
    if !user.allows("mailbox.manage") {
        return Err(AppError::Forbidden(
            "You do not have permission to register a mailbox.".into(),
        ));
    }
    let _ = (state, body);
    Err(AppError::Validation(
        "Create a person to open a mailbox.".into(),
    ))
}

pub async fn update_mailbox(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<Uuid>,
    Body(body): Body<MailboxPatch>,
) -> Result<Json<ApiData<MailboxView>>, AppError> {
    let pool = pool_of(&state)?;
    let current = load_mailbox(pool, id, &user).await?;
    if !current.can_manage {
        return Err(AppError::NotFound);
    }
    if body.display_name.is_none() && body.is_enabled.is_none() {
        return Err(AppError::Validation("Nothing to change.".into()));
    }
    let name = match body.display_name.as_deref() {
        Some(value) => Some(display_name(value)?),
        None => None,
    };
    sqlx::query(
        r#"
        UPDATE mailboxes
        SET display_name = COALESCE($2, display_name),
            is_enabled = COALESCE($3, is_enabled)
        WHERE id = $1
        "#,
    )
    .bind(id)
    .bind(name)
    .bind(body.is_enabled)
    .execute(pool)
    .await?;
    write_audit(
        pool,
        Some(user.id),
        "mailbox.updated",
        Some("mailbox"),
        Some(id),
        serde_json::json!({}),
    )
    .await?;
    let view = load_mailbox(pool, id, &user).await?;
    Ok(Json(ApiData { data: view }))
}

pub async fn delete_mailbox(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    let _ = (state, user, id);
    Err(AppError::Validation(
        "A mailbox stays with its person.".into(),
    ))
}

pub async fn list_members(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiData<Vec<MemberView>>>, AppError> {
    let pool = pool_of(&state)?;
    let mailbox = load_mailbox(pool, id, &user).await?;
    if !mailbox.can_manage {
        return Err(AppError::NotFound);
    }
    let rows = sqlx::query_as::<_, MemberView>(
        r#"
        SELECT u.id, u.email::text AS email, u.display_name,
               mu.can_read, mu.can_send, mu.can_manage, mu.is_default
        FROM mailbox_users mu
        JOIN users u ON u.id = mu.user_id
        WHERE mu.mailbox_id = $1
        ORDER BY u.email
        "#,
    )
    .bind(id)
    .fetch_all(pool)
    .await?;
    Ok(Json(ApiData { data: rows }))
}

pub async fn grant_member(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<Uuid>,
    Body(body): Body<GrantBody>,
) -> Result<Json<ApiData<MemberView>>, AppError> {
    let _ = (state, user, id, body);
    Err(AppError::Validation(
        "A mailbox belongs to one person.".into(),
    ))
}

pub async fn revoke_member(
    State(state): State<AppState>,
    user: CurrentUser,
    Path((id, member_id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode, AppError> {
    let _ = (state, user, id, member_id);
    Err(AppError::Validation(
        "A mailbox belongs to one person.".into(),
    ))
}

pub async fn make_default(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiData<MailboxView>>, AppError> {
    let pool = pool_of(&state)?;
    let mailbox = load_mailbox(pool, id, &user).await?;
    if !mailbox.can_read {
        return Err(AppError::NotFound);
    }
    let mut tx = pool.begin().await?;
    sqlx::query("UPDATE mailbox_users SET is_default = false WHERE user_id = $1")
        .bind(user.id)
        .execute(&mut *tx)
        .await?;
    sqlx::query(
        "UPDATE mailbox_users SET is_default = true WHERE user_id = $1 AND mailbox_id = $2",
    )
    .bind(user.id)
    .bind(id)
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        r#"
        INSERT INTO user_preferences (user_id, default_mailbox_id)
        VALUES ($1, $2)
        ON CONFLICT (user_id) DO UPDATE SET default_mailbox_id = EXCLUDED.default_mailbox_id
        "#,
    )
    .bind(user.id)
    .bind(id)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    let view = load_mailbox(pool, id, &user).await?;
    Ok(Json(ApiData { data: view }))
}

const MAILBOX_LIST: &str = "SELECT m.id, m.address::text AS address, m.display_name, m.is_enabled, m.is_receiving_verified, mu.can_read, mu.can_send, mu.can_manage, mu.is_default, m.created_at FROM mailboxes m JOIN mailbox_users mu ON mu.mailbox_id = m.id AND mu.user_id = $1 WHERE mu.can_read = true ORDER BY m.address";

const MAILBOX_ONE: &str = "SELECT m.id, m.address::text AS address, m.display_name, m.is_enabled, m.is_receiving_verified, mu.can_read, mu.can_send, mu.can_manage, mu.is_default, m.created_at FROM mailboxes m LEFT JOIN mailbox_users mu ON mu.mailbox_id = m.id AND mu.user_id = $1 WHERE m.id = $2";

async fn load_mailbox(
    pool: &PgPool,
    id: Uuid,
    user: &CurrentUser,
) -> Result<MailboxView, AppError> {
    let platform = user.allows("mailbox.manage");
    let row = sqlx::query_as::<_, MailboxRow>(MAILBOX_ONE)
        .bind(user.id)
        .bind(id)
        .fetch_optional(pool)
        .await?;
    let Some(row) = row else {
        return Err(AppError::NotFound);
    };
    if row.can_read.is_none() && !platform {
        return Err(AppError::NotFound);
    }
    Ok(view_from(row, platform))
}

fn view_from(row: MailboxRow, platform: bool) -> MailboxView {
    MailboxView {
        id: row.id,
        address: row.address,
        display_name: row.display_name,
        is_enabled: row.is_enabled,
        is_receiving_verified: row.is_receiving_verified,
        can_read: row.can_read.unwrap_or(false),
        can_send: row.can_send.unwrap_or(false),
        can_manage: row.can_manage.unwrap_or(false) || platform,
        is_default: row.is_default.unwrap_or(false),
        created_at: row.created_at,
    }
}

