use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use crate::api::body::Body;
use crate::api::errors::AppError;
use crate::api::response::ApiData;
use crate::auth::{CurrentUser, display_name, normalize_email, pool_of, unique_or, write_audit};
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
}

pub async fn list_mailboxes(
    State(state): State<AppState>,
    user: CurrentUser,
) -> Result<Json<ApiData<Vec<MailboxView>>>, AppError> {
    let pool = pool_of(&state)?;
    let platform = user.allows("mailbox.manage");
    let rows = sqlx::query_as::<_, MailboxRow>(MAILBOX_LIST)
        .bind(user.id)
        .bind(platform)
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
    let pool = pool_of(&state)?;
    let address = normalize_email(&body.address)?;
    let name = display_name(&body.display_name)?;
    let mut tx = pool.begin().await?;
    let id = match sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO mailboxes (address, display_name) VALUES ($1, $2) RETURNING id",
    )
    .bind(&address)
    .bind(&name)
    .fetch_one(&mut *tx)
    .await
    {
        Ok(id) => id,
        Err(error) => return Err(unique_or(error, "That address is already registered.")),
    };
    sqlx::query("INSERT INTO mailbox_settings (mailbox_id) VALUES ($1)")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    crate::records::ensure_system_folders(&mut tx, id).await?;
    let has_default = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM mailbox_users WHERE user_id = $1 AND is_default)",
    )
    .bind(user.id)
    .fetch_one(&mut *tx)
    .await?;
    sqlx::query(
        r#"
        INSERT INTO mailbox_users
            (mailbox_id, user_id, can_read, can_send, can_manage, is_default)
        VALUES ($1, $2, true, true, true, $3)
        "#,
    )
    .bind(id)
    .bind(user.id)
    .bind(!has_default)
    .execute(&mut *tx)
    .await?;
    if !has_default {
        sqlx::query("UPDATE user_preferences SET default_mailbox_id = $2 WHERE user_id = $1")
            .bind(user.id)
            .bind(id)
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await?;
    write_audit(
        pool,
        Some(user.id),
        "mailbox.created",
        Some("mailbox"),
        Some(id),
        serde_json::json!({ "address": address }),
    )
    .await?;
    let view = load_mailbox(pool, id, &user).await?;
    Ok((StatusCode::CREATED, Json(ApiData { data: view })).into_response())
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
    if !user.allows("mailbox.manage") {
        return Err(AppError::Forbidden(
            "You do not have permission to remove a mailbox.".into(),
        ));
    }
    let pool = pool_of(&state)?;
    let result = sqlx::query("DELETE FROM mailboxes WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    write_audit(
        pool,
        Some(user.id),
        "mailbox.deleted",
        Some("mailbox"),
        Some(id),
        serde_json::json!({}),
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
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
    let pool = pool_of(&state)?;
    let mailbox = load_mailbox(pool, id, &user).await?;
    if !mailbox.can_manage {
        return Err(AppError::NotFound);
    }
    let email = normalize_email(&body.email)?;
    let mut can_read = body.can_read.unwrap_or(true);
    let can_send = body.can_send.unwrap_or(false);
    let can_manage = body.can_manage.unwrap_or(false);
    if can_send || can_manage {
        can_read = true;
    }
    if !can_read && !can_send && !can_manage {
        return Err(AppError::Validation(
            "Choose at least one permission.".into(),
        ));
    }
    let member_id = sqlx::query_scalar::<_, Uuid>("SELECT id FROM users WHERE email = $1")
        .bind(&email)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::Validation("No account uses that email.".into()))?;
    sqlx::query(
        r#"
        INSERT INTO user_preferences (user_id)
        VALUES ($1)
        ON CONFLICT (user_id) DO NOTHING
        "#,
    )
    .bind(member_id)
    .execute(pool)
    .await?;
    sqlx::query(
        r#"
        INSERT INTO mailbox_users (mailbox_id, user_id, can_read, can_send, can_manage)
        VALUES ($1, $2, $3, $4, $5)
        ON CONFLICT (mailbox_id, user_id) DO UPDATE
        SET can_read = EXCLUDED.can_read,
            can_send = EXCLUDED.can_send,
            can_manage = EXCLUDED.can_manage
        "#,
    )
    .bind(id)
    .bind(member_id)
    .bind(can_read)
    .bind(can_send)
    .bind(can_manage)
    .execute(pool)
    .await?;
    write_audit(
        pool,
        Some(user.id),
        "mailbox.member_granted",
        Some("mailbox"),
        Some(id),
        serde_json::json!({ "user_id": member_id }),
    )
    .await?;
    let member = member(pool, id, member_id).await?;
    Ok(Json(ApiData { data: member }))
}

pub async fn revoke_member(
    State(state): State<AppState>,
    user: CurrentUser,
    Path((id, member_id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode, AppError> {
    let pool = pool_of(&state)?;
    let mailbox = load_mailbox(pool, id, &user).await?;
    if !mailbox.can_manage {
        return Err(AppError::NotFound);
    }
    let result = sqlx::query("DELETE FROM mailbox_users WHERE mailbox_id = $1 AND user_id = $2")
        .bind(id)
        .bind(member_id)
        .execute(pool)
        .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    sqlx::query(
        "UPDATE user_preferences SET default_mailbox_id = NULL WHERE user_id = $1 AND default_mailbox_id = $2",
    )
    .bind(member_id)
    .bind(id)
    .execute(pool)
    .await?;
    write_audit(
        pool,
        Some(user.id),
        "mailbox.member_revoked",
        Some("mailbox"),
        Some(id),
        serde_json::json!({ "user_id": member_id }),
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
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

const MAILBOX_LIST: &str = "SELECT m.id, m.address::text AS address, m.display_name, m.is_enabled, m.is_receiving_verified, mu.can_read, mu.can_send, mu.can_manage, mu.is_default FROM mailboxes m LEFT JOIN mailbox_users mu ON mu.mailbox_id = m.id AND mu.user_id = $1 WHERE mu.user_id IS NOT NULL OR $2 ORDER BY m.address";

const MAILBOX_ONE: &str = "SELECT m.id, m.address::text AS address, m.display_name, m.is_enabled, m.is_receiving_verified, mu.can_read, mu.can_send, mu.can_manage, mu.is_default FROM mailboxes m LEFT JOIN mailbox_users mu ON mu.mailbox_id = m.id AND mu.user_id = $1 WHERE m.id = $2";

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
    }
}

async fn member(pool: &PgPool, mailbox_id: Uuid, user_id: Uuid) -> Result<MemberView, AppError> {
    sqlx::query_as::<_, MemberView>(
        r#"
        SELECT u.id, u.email::text AS email, u.display_name,
               mu.can_read, mu.can_send, mu.can_manage, mu.is_default
        FROM mailbox_users mu
        JOIN users u ON u.id = mu.user_id
        WHERE mu.mailbox_id = $1 AND mu.user_id = $2
        "#,
    )
    .bind(mailbox_id)
    .bind(user_id)
    .fetch_one(pool)
    .await
    .map_err(AppError::from)
}
