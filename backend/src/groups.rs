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
    if mailbox_ids.is_empty() {
        return Err(AppError::Validation(
            "Add at least one mailbox to the group.".into(),
        ));
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
            Some("23505") => {
                return AppError::Conflict("A group with that name already exists.".into());
            }
            Some("23503") => {
                return AppError::Validation("Choose mailboxes that exist.".into());
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
}
