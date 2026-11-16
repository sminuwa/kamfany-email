//! Contacts owned by the signed-in person.
//!
//! A contact is not a mailbox grant. Another person cannot read it.

use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::api::AppError;
use crate::auth::{CurrentUser, normalize_email, pool_of, unique_or, write_audit};
use crate::compose::{content_type, inline_preview};
use crate::state::AppState;

const MAX_IMPORT_BYTES: usize = 256 * 1024;
const MAX_IMPORT_ROWS: usize = 500;
const MAX_AVATAR_BYTES: usize = 2 * 1024 * 1024;

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct Contact {
    pub id: Uuid,
    pub full_name: String,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub company: Option<String>,
    pub notes: Option<String>,
    pub has_avatar: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug)]
pub struct ContactInput {
    pub full_name: String,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub company: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ImportResult {
    pub created: i64,
    pub updated: i64,
    pub skipped: i64,
}

#[derive(Debug)]
pub struct Avatar {
    pub content_type: String,
    pub bytes: Vec<u8>,
}

pub async fn list_contacts(
    state: &AppState,
    user: &CurrentUser,
    query: &str,
) -> Result<Vec<Contact>, AppError> {
    let pool = pool_of(state)?;
    let query = query.trim();
    if query.chars().count() > 80 || query.chars().any(|character| character.is_control()) {
        return Ok(Vec::new());
    }
    let pattern = (!query.is_empty()).then(|| like_pattern(query));
    let rows = sqlx::query_as::<_, Contact>(
        r#"
        SELECT id, full_name, email::text AS email, phone, company, notes,
               avatar_storage_key IS NOT NULL AS has_avatar, created_at, updated_at
        FROM contacts
        WHERE owner_user_id = $1
          AND (
                $2::text IS NULL
                OR full_name ILIKE $2 ESCAPE '\'
                OR email::text ILIKE $2 ESCAPE '\'
                OR COALESCE(company, '') ILIKE $2 ESCAPE '\'
                OR COALESCE(phone, '') ILIKE $2 ESCAPE '\'
          )
        ORDER BY lower(full_name), id
        LIMIT 200
        "#,
    )
    .bind(user.id)
    .bind(pattern)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn create_contact(
    state: &AppState,
    user: &CurrentUser,
    input: ContactInput,
) -> Result<Contact, AppError> {
    let pool = pool_of(state)?;
    let fields = clean_input(input)?;
    let id = sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO contacts (owner_user_id, full_name, email, phone, company, notes)
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING id
        "#,
    )
    .bind(user.id)
    .bind(&fields.full_name)
    .bind(&fields.email)
    .bind(&fields.phone)
    .bind(&fields.company)
    .bind(&fields.notes)
    .fetch_one(pool)
    .await
    .map_err(|error| unique_or(error, "A contact with that email already exists."))?;
    load_contact(pool, user.id, id).await
}

pub async fn update_contact(
    state: &AppState,
    user: &CurrentUser,
    contact_id: Uuid,
    input: ContactInput,
) -> Result<Contact, AppError> {
    let pool = pool_of(state)?;
    let fields = clean_input(input)?;
    let updated = sqlx::query(
        r#"
        UPDATE contacts
        SET full_name = $3, email = $4, phone = $5, company = $6, notes = $7
        WHERE id = $1 AND owner_user_id = $2
        "#,
    )
    .bind(contact_id)
    .bind(user.id)
    .bind(&fields.full_name)
    .bind(&fields.email)
    .bind(&fields.phone)
    .bind(&fields.company)
    .bind(&fields.notes)
    .execute(pool)
    .await
    .map_err(|error| unique_or(error, "A contact with that email already exists."))?;
    if updated.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    load_contact(pool, user.id, contact_id).await
}

pub async fn delete_contact(
    state: &AppState,
    user: &CurrentUser,
    contact_id: Uuid,
) -> Result<(), AppError> {
    let pool = pool_of(state)?;
    let key = sqlx::query_scalar::<_, Option<String>>(
        "DELETE FROM contacts WHERE id = $1 AND owner_user_id = $2 RETURNING avatar_storage_key",
    )
    .bind(contact_id)
    .bind(user.id)
    .fetch_optional(pool)
    .await?;
    let Some(key) = key else {
        return Err(AppError::NotFound);
    };
    if let Some(key) = key {
        state.storage.delete(&key).await;
    }
    if let Err(error) = write_audit(
        pool,
        Some(user.id),
        "contact.deleted",
        Some("contact"),
        Some(contact_id),
        serde_json::json!({}),
    )
    .await
    {
        tracing::error!(%error, "audit log was not written");
    }
    Ok(())
}

pub async fn import_contacts(
    state: &AppState,
    user: &CurrentUser,
    bytes: &[u8],
) -> Result<ImportResult, AppError> {
    if bytes.len() > MAX_IMPORT_BYTES {
        return Err(AppError::Validation(
            "A contact import can be at most 256 KB.".into(),
        ));
    }
    let text = std::str::from_utf8(bytes)
        .map_err(|_| AppError::Validation("The import must be UTF-8 text.".into()))?;
    let mut rows = parse_csv(text);
    if rows.len() > MAX_IMPORT_ROWS {
        return Err(AppError::Validation(
            "A contact import can contain at most 500 people.".into(),
        ));
    }
    let pool = pool_of(state)?;
    let mut created = 0_i64;
    let mut updated = 0_i64;
    let mut skipped = 0_i64;
    for row in rows.drain(..) {
        let Some(fields) = clean_input(row).ok() else {
            skipped += 1;
            continue;
        };
        let Some(email) = fields.email.clone() else {
            let inserted = sqlx::query(
                r#"
                INSERT INTO contacts (owner_user_id, full_name, phone, company, notes)
                VALUES ($1, $2, $3, $4, $5)
                "#,
            )
            .bind(user.id)
            .bind(&fields.full_name)
            .bind(&fields.phone)
            .bind(&fields.company)
            .bind(&fields.notes)
            .execute(pool)
            .await?;
            created += i64::try_from(inserted.rows_affected()).unwrap_or(0);
            continue;
        };
        let existing = sqlx::query_scalar::<_, Uuid>(
            "SELECT id FROM contacts WHERE owner_user_id = $1 AND email = $2",
        )
        .bind(user.id)
        .bind(&email)
        .fetch_optional(pool)
        .await?;
        if let Some(id) = existing {
            sqlx::query(
                r#"
                UPDATE contacts
                SET full_name = $3, phone = $4, company = $5, notes = $6
                WHERE id = $1 AND owner_user_id = $2
                "#,
            )
            .bind(id)
            .bind(user.id)
            .bind(&fields.full_name)
            .bind(&fields.phone)
            .bind(&fields.company)
            .bind(&fields.notes)
            .execute(pool)
            .await?;
            updated += 1;
        } else {
            sqlx::query(
                r#"
                INSERT INTO contacts (owner_user_id, full_name, email, phone, company, notes)
                VALUES ($1, $2, $3, $4, $5, $6)
                "#,
            )
            .bind(user.id)
            .bind(&fields.full_name)
            .bind(&email)
            .bind(&fields.phone)
            .bind(&fields.company)
            .bind(&fields.notes)
            .execute(pool)
            .await
            .map_err(|error| unique_or(error, "A contact with that email already exists."))?;
            created += 1;
        }
    }
    Ok(ImportResult {
        created,
        updated,
        skipped,
    })
}

pub async fn save_avatar(
    state: &AppState,
    user: &CurrentUser,
    contact_id: Uuid,
    mime: &str,
    bytes: &[u8],
) -> Result<Contact, AppError> {
    if bytes.len() > MAX_AVATAR_BYTES {
        return Err(AppError::Validation(
            "A profile image can be at most 2 MB.".into(),
        ));
    }
    if bytes.is_empty() {
        return Err(AppError::Validation("The file is empty.".into()));
    }
    let mime = content_type(mime);
    if !inline_preview(&mime) {
        return Err(AppError::Validation(
            "Use a PNG, JPEG, GIF, or WebP image.".into(),
        ));
    }
    let pool = pool_of(state)?;
    let previous = sqlx::query_scalar::<_, Option<String>>(
        "SELECT avatar_storage_key FROM contacts WHERE id = $1 AND owner_user_id = $2",
    )
    .bind(contact_id)
    .bind(user.id)
    .fetch_optional(pool)
    .await?
    .ok_or(AppError::NotFound)?;
    let key = format!("contacts/{contact_id}");
    state.storage.put(&key, bytes).await?;
    let saved = sqlx::query(
        "UPDATE contacts SET avatar_storage_key = $3 WHERE id = $1 AND owner_user_id = $2",
    )
    .bind(contact_id)
    .bind(user.id)
    .bind(&key)
    .execute(pool)
    .await?;
    if saved.rows_affected() == 0 {
        state.storage.delete(&key).await;
        return Err(AppError::NotFound);
    }
    if let Some(previous) = previous.filter(|value| value != &key) {
        state.storage.delete(&previous).await;
    }
    load_contact(pool, user.id, contact_id).await
}

pub async fn remove_avatar(
    state: &AppState,
    user: &CurrentUser,
    contact_id: Uuid,
) -> Result<(), AppError> {
    let pool = pool_of(state)?;
    let key = sqlx::query_scalar::<_, Option<String>>(
        "SELECT avatar_storage_key FROM contacts WHERE id = $1 AND owner_user_id = $2",
    )
    .bind(contact_id)
    .bind(user.id)
    .fetch_optional(pool)
    .await?;
    let Some(key) = key else {
        return Err(AppError::NotFound);
    };
    sqlx::query(
        "UPDATE contacts SET avatar_storage_key = NULL WHERE id = $1 AND owner_user_id = $2",
    )
    .bind(contact_id)
    .bind(user.id)
    .execute(pool)
    .await?;
    if let Some(key) = key {
        state.storage.delete(&key).await;
    }
    Ok(())
}

pub async fn open_avatar(
    state: &AppState,
    user: &CurrentUser,
    contact_id: Uuid,
) -> Result<Avatar, AppError> {
    let pool = pool_of(state)?;
    let key = sqlx::query_scalar::<_, Option<String>>(
        "SELECT avatar_storage_key FROM contacts WHERE id = $1 AND owner_user_id = $2",
    )
    .bind(contact_id)
    .bind(user.id)
    .fetch_optional(pool)
    .await?
    .ok_or(AppError::NotFound)?;
    let Some(key) = key else {
        return Err(AppError::NotFound);
    };
    let bytes = state.storage.get(&key).await?;
    let content_type = if bytes.starts_with(b"\x89PNG") {
        "image/png"
    } else if bytes.starts_with(&[0xff, 0xd8, 0xff]) {
        "image/jpeg"
    } else if bytes.starts_with(b"GIF8") {
        "image/gif"
    } else if bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        "image/webp"
    } else {
        "application/octet-stream"
    };
    Ok(Avatar {
        content_type: content_type.to_string(),
        bytes,
    })
}

async fn load_contact(pool: &PgPool, user_id: Uuid, contact_id: Uuid) -> Result<Contact, AppError> {
    sqlx::query_as::<_, Contact>(
        r#"
        SELECT id, full_name, email::text AS email, phone, company, notes,
               avatar_storage_key IS NOT NULL AS has_avatar, created_at, updated_at
        FROM contacts
        WHERE id = $1 AND owner_user_id = $2
        "#,
    )
    .bind(contact_id)
    .bind(user_id)
    .fetch_optional(pool)
    .await?
    .ok_or(AppError::NotFound)
}

fn clean_input(input: ContactInput) -> Result<ContactInput, AppError> {
    let full_name = input.full_name.trim().to_string();
    if full_name.is_empty() || full_name.chars().count() > 120 {
        return Err(AppError::Validation(
            "Enter a name of 120 characters or fewer.".into(),
        ));
    }
    let email = match input.email.as_deref().map(str::trim).filter(|value| !value.is_empty()) {
        Some(value) => Some(normalize_email(value)?),
        None => None,
    };
    let phone = optional_text(input.phone, 40, "The phone number is too long.")?;
    let company = optional_text(input.company, 120, "The company name is too long.")?;
    let notes = optional_text(input.notes, 4000, "The notes are too long.")?;
    Ok(ContactInput {
        full_name,
        email,
        phone,
        company,
        notes,
    })
}

fn optional_text(value: Option<String>, max: usize, message: &str) -> Result<Option<String>, AppError> {
    let Some(value) = value.map(|item| item.trim().to_string()).filter(|item| !item.is_empty()) else {
        return Ok(None);
    };
    if value.chars().count() > max {
        return Err(AppError::Validation(message.to_string()));
    }
    Ok(Some(value))
}

fn like_pattern(value: &str) -> String {
    let escaped = value
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_");
    format!("%{escaped}%")
}

pub fn parse_csv(text: &str) -> Vec<ContactInput> {
    let mut lines = text.lines().filter(|line| !line.trim().is_empty());
    let Some(first) = lines.next() else {
        return Vec::new();
    };
    let header = split_csv_line(first);
    let named = header
        .first()
        .is_some_and(|cell| cell.eq_ignore_ascii_case("name") || cell.eq_ignore_ascii_case("full name"));
    let mut rows = Vec::new();
    if !named {
        rows.push(row_from(&header));
    }
    for line in lines {
        rows.push(row_from(&split_csv_line(line)));
    }
    rows
}

fn row_from(cells: &[String]) -> ContactInput {
    ContactInput {
        full_name: cells.first().cloned().unwrap_or_default(),
        email: cells.get(1).cloned(),
        phone: cells.get(2).cloned(),
        company: cells.get(3).cloned(),
        notes: cells.get(4).cloned(),
    }
}

fn split_csv_line(line: &str) -> Vec<String> {
    let mut cells = Vec::new();
    let mut current = String::new();
    let mut quoted = false;
    let mut characters = line.chars().peekable();
    while let Some(character) = characters.next() {
        if quoted {
            if character == '"' {
                if characters.peek() == Some(&'"') {
                    current.push('"');
                    characters.next();
                } else {
                    quoted = false;
                }
            } else {
                current.push(character);
            }
        } else if character == '"' && current.is_empty() {
            quoted = true;
        } else if character == ',' {
            cells.push(current.trim().to_string());
            current = String::new();
        } else {
            current.push(character);
        }
    }
    cells.push(current.trim().to_string());
    cells
}

#[cfg(test)]
mod tests {
    use super::parse_csv;

    #[test]
    fn csv_keeps_a_quoted_comma() {
        let rows = parse_csv("name,email,phone,company,notes\n\"Ada, Desk\",ada@kamfany.com,,\"IT KAMFANY\",\"Hello\"\n");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].full_name, "Ada, Desk");
        assert_eq!(rows[0].email.as_deref(), Some("ada@kamfany.com"));
        assert_eq!(rows[0].company.as_deref(), Some("IT KAMFANY"));
    }
}
