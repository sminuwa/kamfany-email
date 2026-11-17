//! Mailbox groups. The owner names a group and adds mailboxes. Sending to the group reaches each member.

use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::api::AppError;
use crate::auth::{display_name, load_roles, write_audit};
use crate::state::AppState;

const MAX_MEMBERS: usize = 50;

#[derive(Debug, Clone, Serialize)]
pub struct GroupMailbox {
    pub id: Uuid,
    pub address: String,
    pub display_name: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct GroupView {
    pub id: Uuid,
    pub name: String,
    pub mailboxes: Vec<GroupMailbox>,
}

#[derive(Debug, Clone)]
pub struct GroupMember {
    pub mailbox_id: Uuid,
    pub address: String,
    pub enabled: bool,
}

#[derive(Debug, Clone)]
pub struct GroupDirectory {
    pub id: Uuid,
    pub name: String,
    pub members: Vec<GroupMember>,
}

#[derive(Debug)]
pub struct Fanout {
    pub to: Vec<String>,
    pub cc: Vec<String>,
    pub bcc: Vec<String>,
    pub mailboxes: Vec<Uuid>,
}

pub struct GroupInput {
    pub name: String,
    pub mailbox_ids: Vec<Uuid>,
}

pub async fn list(state: &AppState) -> Result<Vec<GroupView>, AppError> {
    let pool = crate::auth::pool_of(state)?;
    load_all(pool).await
}

pub async fn create(
    state: &AppState,
    actor: Uuid,
    input: GroupInput,
) -> Result<GroupView, AppError> {
    let pool = crate::auth::pool_of(state)?;
    require_owner(pool, actor).await?;
    let name = display_name(&input.name)?;
    let mailbox_ids = clean_members(&input.mailbox_ids)?;
    let mut tx = pool.begin().await?;
    let id =
        sqlx::query_scalar::<_, Uuid>("INSERT INTO mailbox_groups (name) VALUES ($1) RETURNING id")
            .bind(&name)
            .fetch_one(&mut *tx)
            .await
            .map_err(map_db)?;
    insert_members(&mut tx, id, &mailbox_ids).await?;
    tx.commit().await?;
    write_audit(
        pool,
        Some(actor),
        "group.created",
        Some("mailbox_group"),
        Some(id),
        serde_json::json!({ "name": name, "mailboxes": mailbox_ids.len() }),
    )
    .await?;
    load_one(pool, id).await
}

pub async fn update(
    state: &AppState,
    actor: Uuid,
    id: Uuid,
    name: Option<String>,
    mailbox_ids: Option<Vec<Uuid>>,
) -> Result<GroupView, AppError> {
    let pool = crate::auth::pool_of(state)?;
    require_owner(pool, actor).await?;
    if name.is_none() && mailbox_ids.is_none() {
        return Err(AppError::Validation("Nothing to change.".into()));
    }
    let name = match name {
        Some(value) => Some(display_name(&value)?),
        None => None,
    };
    let mailbox_ids = match mailbox_ids {
        Some(ids) => Some(clean_members(&ids)?),
        None => None,
    };
    load_one(pool, id).await?;
    let mut tx = pool.begin().await?;
    if let Some(name) = &name {
        sqlx::query("UPDATE mailbox_groups SET name = $2 WHERE id = $1")
            .bind(id)
            .bind(name)
            .execute(&mut *tx)
            .await
            .map_err(map_db)?;
    }
    if let Some(mailbox_ids) = &mailbox_ids {
        sqlx::query("DELETE FROM mailbox_group_members WHERE group_id = $1")
            .bind(id)
            .execute(&mut *tx)
            .await?;
        for mailbox_id in mailbox_ids {
            sqlx::query("INSERT INTO mailbox_group_members (group_id, mailbox_id) VALUES ($1, $2)")
                .bind(id)
                .bind(mailbox_id)
                .execute(&mut *tx)
                .await
                .map_err(map_db)?;
        }
    }
    tx.commit().await?;
    write_audit(
        pool,
        Some(actor),
        "group.updated",
        Some("mailbox_group"),
        Some(id),
        serde_json::json!({}),
    )
    .await?;
    load_one(pool, id).await
}

pub async fn delete(state: &AppState, actor: Uuid, id: Uuid) -> Result<(), AppError> {
    let pool = crate::auth::pool_of(state)?;
    require_owner(pool, actor).await?;
    let removed = sqlx::query("DELETE FROM mailbox_groups WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    if removed.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    write_audit(
        pool,
        Some(actor),
        "group.deleted",
        Some("mailbox_group"),
        Some(id),
        serde_json::json!({}),
    )
    .await?;
    Ok(())
}

pub async fn add_mailbox(
    state: &AppState,
    actor: Uuid,
    group_id: Uuid,
    mailbox_id: Uuid,
) -> Result<GroupView, AppError> {
    let pool = crate::auth::pool_of(state)?;
    require_owner(pool, actor).await?;
    load_one(pool, group_id).await?;
    let count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM mailbox_group_members WHERE group_id = $1")
            .bind(group_id)
            .fetch_one(pool)
            .await?;
    if count >= MAX_MEMBERS as i64 {
        return Err(AppError::Validation(
            "A group can include at most 50 mailboxes.".into(),
        ));
    }
    sqlx::query("INSERT INTO mailbox_group_members (group_id, mailbox_id) VALUES ($1, $2)")
        .bind(group_id)
        .bind(mailbox_id)
        .execute(pool)
        .await
        .map_err(map_db)?;
    write_audit(
        pool,
        Some(actor),
        "group.member_added",
        Some("mailbox_group"),
        Some(group_id),
        serde_json::json!({ "mailbox_id": mailbox_id }),
    )
    .await?;
    load_one(pool, group_id).await
}

pub async fn remove_mailbox(
    state: &AppState,
    actor: Uuid,
    group_id: Uuid,
    mailbox_id: Uuid,
) -> Result<GroupView, AppError> {
    let pool = crate::auth::pool_of(state)?;
    require_owner(pool, actor).await?;
    let removed =
        sqlx::query("DELETE FROM mailbox_group_members WHERE group_id = $1 AND mailbox_id = $2")
            .bind(group_id)
            .bind(mailbox_id)
            .execute(pool)
            .await?;
    if removed.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    write_audit(
        pool,
        Some(actor),
        "group.member_removed",
        Some("mailbox_group"),
        Some(group_id),
        serde_json::json!({ "mailbox_id": mailbox_id }),
    )
    .await?;
    load_one(pool, group_id).await
}

#[derive(Debug, Clone, Serialize)]
pub struct GroupImport {
    pub added: i64,
    pub already_in_group: i64,
    pub unknown_count: i64,
    pub unknown: Vec<String>,
    pub not_added: i64,
    pub group: GroupView,
}

pub async fn import_mailboxes(
    state: &AppState,
    actor: Uuid,
    group_id: Uuid,
    filename: &str,
    bytes: &[u8],
) -> Result<GroupImport, AppError> {
    if bytes.len() > 1024 * 1024 {
        return Err(AppError::Validation(
            "An import can be at most 1 MB.".into(),
        ));
    }
    let pool = crate::auth::pool_of(state)?;
    require_owner(pool, actor).await?;
    load_one(pool, group_id).await?;
    let rows = if spreadsheet_file(filename, bytes) {
        spreadsheet_rows(bytes)?
    } else {
        csv_rows(bytes)?
    };
    let addresses = addresses_in_rows(&rows);
    if addresses.is_empty() {
        return Err(AppError::Validation(
            "The file has no mailbox addresses.".into(),
        ));
    }
    if addresses.len() > 500 {
        return Err(AppError::Validation(
            "An import can contain at most 500 addresses.".into(),
        ));
    }
    let found = sqlx::query_as::<_, FoundMailbox>(
        "SELECT id, lower(address::text) AS address FROM mailboxes WHERE lower(address::text) = ANY($1)",
    )
    .bind(&addresses)
    .fetch_all(pool)
    .await?;
    let present: Vec<Uuid> =
        sqlx::query_scalar("SELECT mailbox_id FROM mailbox_group_members WHERE group_id = $1")
            .bind(group_id)
            .fetch_all(pool)
            .await?;
    let mut already_in_group = 0_i64;
    let mut unknown = Vec::new();
    let mut incoming = Vec::new();
    for address in &addresses {
        match found.iter().find(|mailbox| mailbox.address == *address) {
            Some(mailbox) if present.contains(&mailbox.id) || incoming.contains(&mailbox.id) => {
                already_in_group += 1;
            }
            Some(mailbox) => incoming.push(mailbox.id),
            None => unknown.push(address.clone()),
        }
    }
    let room = MAX_MEMBERS.saturating_sub(present.len());
    let not_added = incoming.len().saturating_sub(room) as i64;
    incoming.truncate(room);
    if !incoming.is_empty() {
        let mut tx = pool.begin().await?;
        insert_members(&mut tx, group_id, &incoming).await?;
        tx.commit().await?;
    }
    write_audit(
        pool,
        Some(actor),
        "group.imported",
        Some("mailbox_group"),
        Some(group_id),
        serde_json::json!({ "added": incoming.len(), "unknown": unknown.len() }),
    )
    .await?;
    let unknown_count = unknown.len() as i64;
    unknown.truncate(20);
    Ok(GroupImport {
        added: incoming.len() as i64,
        already_in_group,
        unknown_count,
        unknown,
        not_added,
        group: load_one(pool, group_id).await?,
    })
}

fn spreadsheet_file(filename: &str, bytes: &[u8]) -> bool {
    let name = filename.to_ascii_lowercase();
    bytes.starts_with(b"PK\x03\x04")
        || bytes.starts_with(&[0xD0, 0xCF, 0x11, 0xE0])
        || name.ends_with(".xlsx")
        || name.ends_with(".xlsm")
        || name.ends_with(".xlsb")
        || name.ends_with(".xls")
        || name.ends_with(".ods")
}

fn spreadsheet_rows(bytes: &[u8]) -> Result<Vec<Vec<String>>, AppError> {
    use calamine::{DataType, Reader};
    let cursor = std::io::Cursor::new(bytes);
    let mut workbook = calamine::open_workbook_auto_from_rs(cursor)
        .map_err(|_| AppError::Validation("The spreadsheet could not be read.".into()))?;
    let range = workbook
        .worksheet_range_at(0)
        .ok_or_else(|| AppError::Validation("The spreadsheet has no sheet.".into()))?
        .map_err(|_| AppError::Validation("The spreadsheet could not be read.".into()))?;
    let mut rows = Vec::new();
    for row in range.rows() {
        if row.iter().all(DataType::is_empty) {
            continue;
        }
        rows.push(row.iter().map(cell_text).collect());
        if rows.len() > 500 {
            return Err(AppError::Validation(
                "An import can contain at most 500 rows.".into(),
            ));
        }
    }
    Ok(rows)
}

fn cell_text(cell: &calamine::Data) -> String {
    match cell {
        calamine::Data::String(value) | calamine::Data::DateTimeIso(value) => {
            value.trim().to_string()
        }
        calamine::Data::Int(value) => value.to_string(),
        calamine::Data::Float(value) => value.to_string(),
        calamine::Data::Bool(value) => value.to_string(),
        calamine::Data::DateTime(_)
        | calamine::Data::DurationIso(_)
        | calamine::Data::Error(_)
        | calamine::Data::Empty => String::new(),
    }
}

fn csv_rows(bytes: &[u8]) -> Result<Vec<Vec<String>>, AppError> {
    let text = std::str::from_utf8(bytes)
        .map_err(|_| AppError::Validation("The CSV must be UTF-8 text.".into()))?;
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut rows = Vec::new();
    for line in text.lines() {
        if line.trim().is_empty() {
            continue;
        }
        rows.push(split_csv_line(line));
        if rows.len() > 500 {
            return Err(AppError::Validation(
                "An import can contain at most 500 rows.".into(),
            ));
        }
    }
    Ok(rows)
}

fn addresses_in_rows(rows: &[Vec<String>]) -> Vec<String> {
    let Some(first) = rows.first() else {
        return Vec::new();
    };
    let column = first.iter().position(|cell| {
        matches!(
            cell.trim().to_ascii_lowercase().as_str(),
            "email" | "e-mail" | "address" | "mailbox" | "mail"
        )
    });
    let data = if column.is_some() { &rows[1..] } else { rows };
    let mut addresses = Vec::new();
    for row in data {
        if let Some(index) = column {
            if let Some(cell) = row.get(index) {
                push_address(&mut addresses, cell);
            }
        } else {
            for cell in row {
                push_address(&mut addresses, cell);
            }
        }
    }
    addresses
}

fn push_address(addresses: &mut Vec<String>, cell: &str) {
    let cell = cell.trim();
    if cell.is_empty() {
        return;
    }
    let candidate = cell
        .rfind('<')
        .zip(cell.rfind('>'))
        .filter(|(start, end)| end > start)
        .map(|(start, end)| cell[start + 1..end].trim())
        .unwrap_or(cell);
    let Ok(email) = crate::auth::normalize_email(candidate) else {
        return;
    };
    if !addresses.iter().any(|item| item == &email) {
        addresses.push(email);
    }
}

fn split_csv_line(line: &str) -> Vec<String> {
    let separator = if line.contains(';') && !line.contains(',') {
        ';'
    } else {
        ','
    };
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
        } else if character == separator {
            cells.push(current.trim().to_string());
            current = String::new();
        } else {
            current.push(character);
        }
    }
    cells.push(current.trim().to_string());
    cells
}

pub async fn fanout(
    pool: &PgPool,
    to: &[String],
    cc: &[String],
    bcc: &[String],
    group_ids: &[Uuid],
) -> Result<Fanout, AppError> {
    if group_ids.is_empty() {
        return Ok(Fanout {
            to: to.to_vec(),
            cc: cc.to_vec(),
            bcc: bcc.to_vec(),
            mailboxes: Vec::new(),
        });
    }
    let mut ids = group_ids.to_vec();
    ids.sort();
    ids.dedup();
    let directory = load_directory(pool, &ids).await?;
    if directory.len() != ids.len() {
        return Err(AppError::Validation("Choose a group that exists.".into()));
    }
    expand_recipients(to, cc, bcc, &directory)
}

pub fn expand_recipients(
    to: &[String],
    cc: &[String],
    bcc: &[String],
    groups: &[GroupDirectory],
) -> Result<Fanout, AppError> {
    if groups.is_empty() {
        return Ok(Fanout {
            to: to.to_vec(),
            cc: cc.to_vec(),
            bcc: bcc.to_vec(),
            mailboxes: Vec::new(),
        });
    }
    let mut expanded = to.to_vec();
    let mut mailboxes = Vec::new();
    for group in groups {
        let enabled: Vec<_> = group
            .members
            .iter()
            .filter(|member| member.enabled)
            .collect();
        if enabled.is_empty() {
            return Err(AppError::Validation(format!(
                "{} has no mailboxes that can receive mail.",
                group.name
            )));
        }
        for member in enabled {
            push_unique(&mut expanded, member.address.clone());
            if !mailboxes.contains(&member.mailbox_id) {
                mailboxes.push(member.mailbox_id);
            }
        }
    }
    let mut cc = cc.to_vec();
    let mut bcc = bcc.to_vec();
    cc.retain(|address| !contains_address(&expanded, address));
    bcc.retain(|address| !contains_address(&expanded, address) && !contains_address(&cc, address));
    Ok(Fanout {
        to: expanded,
        cc,
        bcc,
        mailboxes,
    })
}

async fn require_owner(pool: &PgPool, user_id: Uuid) -> Result<(), AppError> {
    let roles = load_roles(pool, user_id).await?;
    if roles.iter().any(|role| role == "owner") {
        Ok(())
    } else {
        Err(AppError::Forbidden(
            "Only the owner can manage groups.".into(),
        ))
    }
}

fn clean_members(ids: &[Uuid]) -> Result<Vec<Uuid>, AppError> {
    let mut mailbox_ids = Vec::new();
    for id in ids {
        if !mailbox_ids.contains(id) {
            mailbox_ids.push(*id);
        }
    }
    if mailbox_ids.len() > MAX_MEMBERS {
        return Err(AppError::Validation(
            "A group can include at most 50 mailboxes.".into(),
        ));
    }
    Ok(mailbox_ids)
}

async fn insert_members(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    group_id: Uuid,
    mailbox_ids: &[Uuid],
) -> Result<(), AppError> {
    for mailbox_id in mailbox_ids {
        sqlx::query("INSERT INTO mailbox_group_members (group_id, mailbox_id) VALUES ($1, $2)")
            .bind(group_id)
            .bind(mailbox_id)
            .execute(&mut **tx)
            .await
            .map_err(map_db)?;
    }
    Ok(())
}

async fn load_all(pool: &PgPool) -> Result<Vec<GroupView>, AppError> {
    let groups = sqlx::query_as::<_, GroupRow>(
        "SELECT id, name FROM mailbox_groups ORDER BY lower(name), name",
    )
    .fetch_all(pool)
    .await?;
    let members = sqlx::query_as::<_, MemberRow>(
        r#"
        SELECT gm.group_id, m.id, m.address::text AS address, m.display_name
        FROM mailbox_group_members gm
        JOIN mailboxes m ON m.id = gm.mailbox_id
        ORDER BY m.address
        "#,
    )
    .fetch_all(pool)
    .await?;
    Ok(groups
        .into_iter()
        .map(|group| GroupView {
            mailboxes: members
                .iter()
                .filter(|member| member.group_id == group.id)
                .map(|member| GroupMailbox {
                    id: member.id,
                    address: member.address.clone(),
                    display_name: member.display_name.clone(),
                })
                .collect(),
            id: group.id,
            name: group.name,
        })
        .collect())
}

async fn load_one(pool: &PgPool, id: Uuid) -> Result<GroupView, AppError> {
    load_all(pool)
        .await?
        .into_iter()
        .find(|group| group.id == id)
        .ok_or(AppError::NotFound)
}

async fn load_directory(pool: &PgPool, ids: &[Uuid]) -> Result<Vec<GroupDirectory>, AppError> {
    let rows = sqlx::query_as::<_, DirectoryRow>(
        r#"
        SELECT g.id, g.name, m.id AS mailbox_id, m.address::text AS mailbox_address, m.is_enabled
        FROM mailbox_groups g
        LEFT JOIN mailbox_group_members gm ON gm.group_id = g.id
        LEFT JOIN mailboxes m ON m.id = gm.mailbox_id
        WHERE g.id = ANY($1)
        ORDER BY g.name, m.address
        "#,
    )
    .bind(ids)
    .fetch_all(pool)
    .await?;
    let mut groups = Vec::<GroupDirectory>::new();
    for row in rows {
        if let Some(group) = groups.iter_mut().find(|group| group.id == row.id) {
            if let (Some(mailbox_id), Some(address), Some(enabled)) =
                (row.mailbox_id, row.mailbox_address, row.is_enabled)
            {
                group.members.push(GroupMember {
                    mailbox_id,
                    address,
                    enabled,
                });
            }
        } else {
            let mut members = Vec::new();
            if let (Some(mailbox_id), Some(address), Some(enabled)) =
                (row.mailbox_id, row.mailbox_address, row.is_enabled)
            {
                members.push(GroupMember {
                    mailbox_id,
                    address,
                    enabled,
                });
            }
            groups.push(GroupDirectory {
                id: row.id,
                name: row.name,
                members,
            });
        }
    }
    Ok(groups)
}

fn push_unique(addresses: &mut Vec<String>, address: String) {
    if !contains_address(addresses, &address) {
        addresses.push(address);
    }
}

fn contains_address(addresses: &[String], address: &str) -> bool {
    addresses
        .iter()
        .any(|item| item.eq_ignore_ascii_case(address))
}

fn map_db(error: sqlx::Error) -> AppError {
    if let sqlx::Error::Database(db) = &error {
        match db.code().as_deref() {
            Some("23505") if db.constraint() == Some("mailbox_group_members_pkey") => {
                return AppError::Conflict("That mailbox is already in the group.".into());
            }
            Some("23505") => {
                return AppError::Conflict("A group with that name already exists.".into());
            }
            Some("23503") => {
                return AppError::Validation("Choose a mailbox that exists.".into());
            }
            _ => {}
        }
    }
    AppError::Database(error)
}

#[derive(sqlx::FromRow)]
struct GroupRow {
    id: Uuid,
    name: String,
}

#[derive(sqlx::FromRow)]
struct MemberRow {
    group_id: Uuid,
    id: Uuid,
    address: String,
    display_name: String,
}

#[derive(sqlx::FromRow)]
struct FoundMailbox {
    id: Uuid,
    address: String,
}

#[derive(sqlx::FromRow)]
struct DirectoryRow {
    id: Uuid,
    name: String,
    mailbox_id: Option<Uuid>,
    mailbox_address: Option<String>,
    is_enabled: Option<bool>,
}

#[cfg(test)]
mod tests {
    use super::{GroupDirectory, GroupMember, expand_recipients};
    use uuid::Uuid;

    fn support() -> GroupDirectory {
        GroupDirectory {
            id: Uuid::from_u128(9),
            name: "Support".into(),
            members: vec![
                GroupMember {
                    mailbox_id: Uuid::from_u128(1),
                    address: "ada@kamfany.com".into(),
                    enabled: true,
                },
                GroupMember {
                    mailbox_id: Uuid::from_u128(2),
                    address: "bola@kamfany.com".into(),
                    enabled: false,
                },
                GroupMember {
                    mailbox_id: Uuid::from_u128(3),
                    address: "chi@kamfany.com".into(),
                    enabled: true,
                },
            ],
        }
    }

    #[test]
    fn a_group_adds_its_enabled_mailboxes() {
        let fanout = expand_recipients(
            &["ada@kamfany.com".into()],
            &["chi@kamfany.com".into()],
            &[],
            &[support()],
        )
        .expect("fanout");
        assert_eq!(
            fanout.to,
            vec!["ada@kamfany.com".to_string(), "chi@kamfany.com".to_string()]
        );
        assert!(fanout.cc.is_empty());
        assert_eq!(fanout.mailboxes.len(), 2);
    }

    #[test]
    fn an_empty_group_cannot_receive_mail() {
        let group = GroupDirectory {
            id: Uuid::from_u128(4),
            name: "Empty".into(),
            members: Vec::new(),
        };
        let error = expand_recipients(&[], &[], &[], &[group]).expect_err("empty");
        assert!(error.to_string().contains("Empty"));
    }

    #[test]
    fn a_csv_uses_the_email_column_and_skips_the_header() {
        let rows = super::csv_rows(
            "name,email\nAda,ada@kamfany.com\n\"Desk, Lead\",ada@kamfany.com\n".as_bytes(),
        )
        .expect("csv");
        assert_eq!(
            super::addresses_in_rows(&rows),
            vec!["ada@kamfany.com".to_string()]
        );
    }

    #[test]
    fn an_excel_file_uses_the_email_column() {
        let rows =
            super::spreadsheet_rows(include_bytes!("../tests/fixtures/group-mailboxes.xlsx"))
                .expect("xlsx");
        assert_eq!(
            super::addresses_in_rows(&rows),
            vec!["ada@kamfany.com".to_string()]
        );
    }

    #[test]
    fn a_plain_list_of_addresses_is_accepted() {
        let rows = super::csv_rows("ada@kamfany.com\nnot-an-email\n".as_bytes()).expect("csv");
        assert_eq!(
            super::addresses_in_rows(&rows),
            vec!["ada@kamfany.com".to_string()]
        );
    }
}
