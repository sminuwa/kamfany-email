use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::api::AppError;
use crate::auth::{CurrentUser, unique_or};

use super::access::require_reader;

#[derive(Debug, Serialize)]
pub struct FolderItem {
    pub id: Uuid,
    pub system_key: Option<String>,
    pub name: String,
    pub color: Option<String>,
    pub sort_order: i32,
    pub unread: i64,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct LabelItem {
    pub id: Uuid,
    pub name: String,
    pub color: Option<String>,
}

pub async fn list_folders(
    pool: &PgPool,
    mailbox_id: Uuid,
    user: &CurrentUser,
) -> Result<Vec<FolderItem>, AppError> {
    require_reader(pool, mailbox_id, user).await?;
    let mut connection = pool.acquire().await?;
    crate::records::ensure_system_folders(&mut connection, mailbox_id).await?;
    let folders = sqlx::query_as::<_, FolderRow>(
        r#"
        SELECT id, system_key, name, color, sort_order
        FROM folders
        WHERE mailbox_id = $1
        ORDER BY sort_order, lower(name)
        "#,
    )
    .bind(mailbox_id)
    .fetch_all(pool)
    .await?;
    let unread = sqlx::query_as::<_, UnreadRow>(
        r#"
        SELECT a.folder_id,
               count(*) FILTER (
                   WHERE NOT m.is_read
                     AND (
                         f.system_key IS DISTINCT FROM 'inbox'
                         OR m.snoozed_until IS NULL
                         OR m.snoozed_until <= now()
                     )
               ) AS unread
        FROM message_folder_assignments a
        JOIN messages m ON m.id = a.message_id
        JOIN folders f ON f.id = a.folder_id
        WHERE m.mailbox_id = $1
        GROUP BY a.folder_id
        "#,
    )
    .bind(mailbox_id)
    .fetch_all(pool)
    .await?;
    let starred = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT count(*)
        FROM messages m
        LEFT JOIN message_folder_assignments a ON a.message_id = m.id
        LEFT JOIN folders f ON f.id = a.folder_id
        WHERE m.mailbox_id = $1
          AND m.is_starred
          AND NOT m.is_read
          AND COALESCE(f.system_key, '') <> 'trash'
        "#,
    )
    .bind(mailbox_id)
    .fetch_one(pool)
    .await?;
    let snoozed = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT count(*)
        FROM messages m
        LEFT JOIN message_folder_assignments a ON a.message_id = m.id
        LEFT JOIN folders f ON f.id = a.folder_id
        WHERE m.mailbox_id = $1
          AND NOT m.is_read
          AND m.snoozed_until > now()
          AND COALESCE(f.system_key, '') <> 'trash'
        "#,
    )
    .bind(mailbox_id)
    .fetch_one(pool)
    .await?;
    Ok(folders
        .into_iter()
        .map(|folder| {
            let unread = match folder.system_key.as_deref() {
                Some("starred") => starred,
                Some("snoozed") => snoozed,
                _ => unread
                    .iter()
                    .find(|row| row.folder_id == folder.id)
                    .map(|row| row.unread)
                    .unwrap_or(0),
            };
            FolderItem {
                id: folder.id,
                system_key: folder.system_key,
                name: folder.name,
                color: folder.color,
                sort_order: folder.sort_order,
                unread,
            }
        })
        .collect())
}

pub async fn create_folder(
    pool: &PgPool,
    mailbox_id: Uuid,
    user: &CurrentUser,
    name: &str,
    color: Option<&str>,
) -> Result<FolderItem, AppError> {
    require_reader(pool, mailbox_id, user).await?;
    let name = clean_name(name, "folder")?;
    let color = clean_color(color)?;
    let id = sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO folders (mailbox_id, name, color, sort_order, created_by)
        VALUES ($1, $2, $3, 100, $4)
        RETURNING id
        "#,
    )
    .bind(mailbox_id)
    .bind(&name)
    .bind(&color)
    .bind(user.id)
    .fetch_one(pool)
    .await
    .map_err(|error| unique_or(error, "A folder with that name already exists."))?;
    Ok(FolderItem {
        id,
        system_key: None,
        name,
        color,
        sort_order: 100,
        unread: 0,
    })
}

pub async fn update_folder(
    pool: &PgPool,
    folder_id: Uuid,
    user: &CurrentUser,
    name: Option<&str>,
    color: Option<&str>,
) -> Result<FolderItem, AppError> {
    let mailbox_id = folder_mailbox(pool, folder_id).await?;
    require_reader(pool, mailbox_id, user).await?;
    let current = load_folder(pool, folder_id).await?;
    if current.system_key.is_some() {
        return Err(AppError::Validation(
            "System folders cannot be changed.".into(),
        ));
    }
    if name.is_none() && color.is_none() {
        return Err(AppError::Validation("Nothing to change.".into()));
    }
    let name = match name {
        Some(value) => clean_name(value, "folder")?,
        None => current.name,
    };
    let color = match color {
        Some(value) => clean_color(Some(value))?,
        None => current.color,
    };
    sqlx::query("UPDATE folders SET name = $2, color = $3 WHERE id = $1")
        .bind(folder_id)
        .bind(&name)
        .bind(&color)
        .execute(pool)
        .await
        .map_err(|error| unique_or(error, "A folder with that name already exists."))?;
    Ok(FolderItem {
        id: folder_id,
        system_key: None,
        name,
        color,
        sort_order: current.sort_order,
        unread: 0,
    })
}

pub async fn delete_folder(
    pool: &PgPool,
    folder_id: Uuid,
    user: &CurrentUser,
) -> Result<(), AppError> {
    let mailbox_id = folder_mailbox(pool, folder_id).await?;
    require_reader(pool, mailbox_id, user).await?;
    let current = load_folder(pool, folder_id).await?;
    if current.system_key.is_some() {
        return Err(AppError::Validation(
            "System folders cannot be removed.".into(),
        ));
    }
    let held = sqlx::query_scalar::<_, i64>(
        "SELECT count(*) FROM message_folder_assignments WHERE folder_id = $1",
    )
    .bind(folder_id)
    .fetch_one(pool)
    .await?;
    if held > 0 {
        return Err(AppError::Validation(
            "Move the messages out of this folder first.".into(),
        ));
    }
    sqlx::query("DELETE FROM folders WHERE id = $1")
        .bind(folder_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn list_labels(
    pool: &PgPool,
    mailbox_id: Uuid,
    user: &CurrentUser,
) -> Result<Vec<LabelItem>, AppError> {
    require_reader(pool, mailbox_id, user).await?;
    sqlx::query_as::<_, LabelItem>(
        r#"
        SELECT id, name, color
        FROM labels
        WHERE mailbox_id = $1
        ORDER BY lower(name)
        "#,
    )
    .bind(mailbox_id)
    .fetch_all(pool)
    .await
    .map_err(AppError::from)
}

pub async fn create_label(
    pool: &PgPool,
    mailbox_id: Uuid,
    user: &CurrentUser,
    name: &str,
    color: Option<&str>,
) -> Result<LabelItem, AppError> {
    require_reader(pool, mailbox_id, user).await?;
    let name = clean_name(name, "label")?;
    let color = clean_color(color)?;
    let id = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO labels (mailbox_id, name, color) VALUES ($1, $2, $3) RETURNING id",
    )
    .bind(mailbox_id)
    .bind(&name)
    .bind(&color)
    .fetch_one(pool)
    .await
    .map_err(|error| unique_or(error, "A label with that name already exists."))?;
    Ok(LabelItem { id, name, color })
}

pub async fn update_label(
    pool: &PgPool,
    label_id: Uuid,
    user: &CurrentUser,
    name: Option<&str>,
    color: Option<&str>,
) -> Result<LabelItem, AppError> {
    let mailbox_id = label_mailbox(pool, label_id).await?;
    require_reader(pool, mailbox_id, user).await?;
    let current = sqlx::query_as::<_, LabelItem>("SELECT id, name, color FROM labels WHERE id = $1")
        .bind(label_id)
        .fetch_optional(pool)
        .await?
        .ok_or(AppError::NotFound)?;
    if name.is_none() && color.is_none() {
        return Err(AppError::Validation("Nothing to change.".into()));
    }
    let name = match name {
        Some(value) => clean_name(value, "label")?,
        None => current.name,
    };
    let color = match color {
        Some(value) => clean_color(Some(value))?,
        None => current.color,
    };
    sqlx::query("UPDATE labels SET name = $2, color = $3 WHERE id = $1")
        .bind(label_id)
        .bind(&name)
        .bind(&color)
        .execute(pool)
        .await
        .map_err(|error| unique_or(error, "A label with that name already exists."))?;
    Ok(LabelItem {
        id: label_id,
        name,
        color,
    })
}

pub async fn delete_label(
    pool: &PgPool,
    label_id: Uuid,
    user: &CurrentUser,
) -> Result<(), AppError> {
    let mailbox_id = label_mailbox(pool, label_id).await?;
    require_reader(pool, mailbox_id, user).await?;
    let removed = sqlx::query("DELETE FROM labels WHERE id = $1")
        .bind(label_id)
        .execute(pool)
        .await?;
    if removed.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    Ok(())
}

pub(super) fn clean_name(value: &str, kind: &str) -> Result<String, AppError> {
    let name = value.trim();
    if name.is_empty() || name.chars().count() > 80 || name.chars().any(char::is_control) {
        return Err(AppError::Validation(format!(
            "Enter a {kind} name up to 80 characters."
        )));
    }
    Ok(name.to_string())
}

pub(super) fn clean_color(value: Option<&str>) -> Result<Option<String>, AppError> {
    match value.map(str::trim).filter(|item| !item.is_empty()) {
        None => Ok(None),
        Some(color @ ("brass" | "paper" | "mist")) => Ok(Some(color.to_string())),
        Some(_) => Err(AppError::Validation(
            "Choose brass, paper, or mist.".into(),
        )),
    }
}

async fn folder_mailbox(pool: &PgPool, folder_id: Uuid) -> Result<Uuid, AppError> {
    sqlx::query_scalar("SELECT mailbox_id FROM folders WHERE id = $1")
        .bind(folder_id)
        .fetch_optional(pool)
        .await?
        .ok_or(AppError::NotFound)
}

async fn label_mailbox(pool: &PgPool, label_id: Uuid) -> Result<Uuid, AppError> {
    sqlx::query_scalar("SELECT mailbox_id FROM labels WHERE id = $1")
        .bind(label_id)
        .fetch_optional(pool)
        .await?
        .ok_or(AppError::NotFound)
}

async fn load_folder(pool: &PgPool, folder_id: Uuid) -> Result<FolderRow, AppError> {
    sqlx::query_as::<_, FolderRow>(
        "SELECT id, system_key, name, color, sort_order FROM folders WHERE id = $1",
    )
    .bind(folder_id)
    .fetch_optional(pool)
    .await?
    .ok_or(AppError::NotFound)
}

#[derive(sqlx::FromRow)]
struct FolderRow {
    id: Uuid,
    system_key: Option<String>,
    name: String,
    color: Option<String>,
    sort_order: i32,
}

#[derive(sqlx::FromRow)]
struct UnreadRow {
    folder_id: Uuid,
    unread: i64,
}
