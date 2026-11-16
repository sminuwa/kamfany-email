use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use crate::api::body::Body;
use crate::api::errors::AppError;
use crate::api::response::ApiData;
use crate::auth::{
    CurrentUser, display_name, hash_password, load_roles, normalize_email, password_policy,
    pool_of, write_audit,
};
use crate::state::AppState;

const ROLE_ADMINISTRATOR: Uuid = uuid::uuid!("10000000-0000-4000-8000-000000000002");
const ROLE_MEMBER: Uuid = uuid::uuid!("10000000-0000-4000-8000-000000000003");

#[derive(Serialize)]
pub struct Person {
    pub id: Uuid,
    pub email: String,
    pub display_name: String,
    pub roles: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub mailbox_id: Uuid,
    pub is_enabled: bool,
    pub is_receiving_verified: bool,
}

#[derive(Deserialize)]
pub struct CreatePerson {
    email: String,
    display_name: String,
    password: String,
    role: String,
}

#[derive(FromRow)]
struct PersonRow {
    id: Uuid,
    email: String,
    display_name: String,
    created_at: DateTime<Utc>,
    mailbox_id: Uuid,
    is_enabled: bool,
    is_receiving_verified: bool,
}

#[derive(Deserialize)]
pub struct UpdatePerson {
    role: String,
}

pub async fn list_users(
    State(state): State<AppState>,
    user: CurrentUser,
) -> Result<Json<ApiData<Vec<Person>>>, AppError> {
    if !user.allows("users.manage") {
        return Err(AppError::Forbidden(
            "You do not have permission to manage accounts.".into(),
        ));
    }
    let pool = pool_of(&state)?;
    let rows = sqlx::query_as::<_, PersonRow>(
        r#"
        SELECT users.id, users.email::text AS email, users.display_name, users.created_at,
               mailboxes.id AS mailbox_id, mailboxes.is_enabled, mailboxes.is_receiving_verified
        FROM users
        JOIN mailboxes ON mailboxes.address = users.email
        ORDER BY users.email
        "#,
    )
    .fetch_all(pool)
    .await?;
    let mut people = Vec::with_capacity(rows.len());
    for row in rows {
        people.push(person_from(pool, row).await?);
    }
    Ok(Json(ApiData { data: people }))
}

pub async fn create_user(
    State(state): State<AppState>,
    user: CurrentUser,
    Body(body): Body<CreatePerson>,
) -> Result<Response, AppError> {
    if !user.allows("users.manage") {
        return Err(AppError::Forbidden(
            "You do not have permission to manage accounts.".into(),
        ));
    }
    let pool = pool_of(&state)?;
    let email = normalize_email(&body.email)?;
    let name = display_name(&body.display_name)?;
    password_policy(&body.password)?;
    let role_id = match body.role.as_str() {
        "administrator" => ROLE_ADMINISTRATOR,
        "member" => ROLE_MEMBER,
        _ => {
            return Err(AppError::Validation(
                "Choose member or administrator.".into(),
            ));
        }
    };
    let password_hash = hash_password(&body.password)?;
    let mut tx = pool.begin().await?;
    let created = match sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, display_name)
        VALUES ($1, $2, $3)
        RETURNING id
        "#,
    )
    .bind(&email)
    .bind(&password_hash)
    .bind(&name)
    .fetch_one(&mut *tx)
    .await
    {
        Ok(created) => created,
        Err(error) => {
            return Err(crate::auth::unique_or(
                error,
                "An account with that email already exists.",
            ));
        }
    };
    let id = created;
    sqlx::query("INSERT INTO user_roles (user_id, role_id) VALUES ($1, $2)")
        .bind(id)
        .bind(role_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO user_preferences (user_id) VALUES ($1)")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    crate::records::open_person_mailbox(&mut tx, id, &email, &name).await?;
    tx.commit().await?;
    write_audit(
        pool,
        Some(user.id),
        "user.created",
        Some("user"),
        Some(id),
        serde_json::json!({ "role": body.role }),
    )
    .await?;
    let person = load_person(pool, id).await?;
    Ok((StatusCode::CREATED, Json(ApiData { data: person })).into_response())
}

pub async fn update_user(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<Uuid>,
    Body(body): Body<UpdatePerson>,
) -> Result<Json<ApiData<Person>>, AppError> {
    if !user.allows("users.manage") {
        return Err(AppError::Forbidden(
            "You do not have permission to manage accounts.".into(),
        ));
    }
    if id == user.id {
        return Err(AppError::Validation("Choose another account.".into()));
    }
    let role_id = match body.role.as_str() {
        "administrator" => ROLE_ADMINISTRATOR,
        "member" => ROLE_MEMBER,
        _ => {
            return Err(AppError::Validation(
                "Choose member or administrator.".into(),
            ));
        }
    };
    let pool = pool_of(&state)?;
    let current = load_roles(pool, id).await?;
    if current.is_empty() {
        return Err(AppError::NotFound);
    }
    if current.iter().any(|role| role == "owner") {
        return Err(AppError::Validation(
            "The owner role stays with the first account.".into(),
        ));
    }
    let mut tx = pool.begin().await?;
    sqlx::query("DELETE FROM user_roles WHERE user_id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO user_roles (user_id, role_id) VALUES ($1, $2)")
        .bind(id)
        .bind(role_id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    write_audit(
        pool,
        Some(user.id),
        "user.role_changed",
        Some("user"),
        Some(id),
        serde_json::json!({ "role": body.role }),
    )
    .await?;
    Ok(Json(ApiData {
        data: load_person(pool, id).await?,
    }))
}

async fn load_person(pool: &sqlx::PgPool, id: Uuid) -> Result<Person, AppError> {
    let row = sqlx::query_as::<_, PersonRow>(
        r#"
        SELECT users.id, users.email::text AS email, users.display_name, users.created_at,
               mailboxes.id AS mailbox_id, mailboxes.is_enabled, mailboxes.is_receiving_verified
        FROM users
        JOIN mailboxes ON mailboxes.address = users.email
        WHERE users.id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    let Some(row) = row else {
        return Err(AppError::NotFound);
    };
    person_from(pool, row).await
}

async fn person_from(pool: &sqlx::PgPool, row: PersonRow) -> Result<Person, AppError> {
    Ok(Person {
        roles: load_roles(pool, row.id).await?,
        id: row.id,
        email: row.email,
        display_name: row.display_name,
        created_at: row.created_at,
        mailbox_id: row.mailbox_id,
        is_enabled: row.is_enabled,
        is_receiving_verified: row.is_receiving_verified,
    })
}
