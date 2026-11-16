use sqlx::PgPool;
use uuid::Uuid;

use crate::auth::{hash_password, normalize_email, password_policy};
use crate::config::{AppConfig, Environment};

const ROLE_OWNER: Uuid = uuid::uuid!("10000000-0000-4000-8000-000000000001");

const DESKS: &[(&str, &str)] = &[
    ("support@kamfany.com", "KAMFANY Support"),
    ("finance@kamfany.com", "KAMFANY Finance"),
    ("admin@kamfany.com", "KAMFANY Admin"),
    ("developer@kamfany.com", "KAMFANY Development"),
    ("info@kamfany.com", "KAMFANY Info"),
];

pub async fn ensure_dev_admin(pool: &PgPool, config: &AppConfig) -> anyhow::Result<()> {
    if config.env != Environment::Development {
        return Ok(());
    }
    let Some(email_raw) = std::env::var("BOOTSTRAP_ADMIN_EMAIL")
        .ok()
        .filter(|value| !value.trim().is_empty())
    else {
        return Ok(());
    };
    let Some(password) = std::env::var("BOOTSTRAP_ADMIN_PASSWORD")
        .ok()
        .filter(|value| !value.is_empty())
    else {
        tracing::warn!("BOOTSTRAP_ADMIN_EMAIL is set without a password");
        return Ok(());
    };
    let Ok(email) = normalize_email(&email_raw) else {
        tracing::warn!("BOOTSTRAP_ADMIN_EMAIL is not a valid address");
        return Ok(());
    };
    if password_policy(&password).is_err() {
        tracing::warn!("BOOTSTRAP_ADMIN_PASSWORD must be 12 to 128 characters");
        return Ok(());
    }

    let existing = sqlx::query_scalar::<_, Uuid>("SELECT id FROM users WHERE email = $1")
        .bind(&email)
        .fetch_optional(pool)
        .await?;
    let user_id = if let Some(id) = existing {
        id
    } else {
        let hash = hash_password(&password).map_err(|_| anyhow::anyhow!("password hash failed"))?;
        let id = sqlx::query_scalar::<_, Uuid>(
            r#"
            INSERT INTO users (email, password_hash, display_name)
            VALUES ($1, $2, 'Adaeze Okonkwo')
            RETURNING id
            "#,
        )
        .bind(&email)
        .bind(hash)
        .fetch_one(pool)
        .await?;
        sqlx::query("INSERT INTO user_roles (user_id, role_id) VALUES ($1, $2)")
            .bind(id)
            .bind(ROLE_OWNER)
            .execute(pool)
            .await?;
        sqlx::query("INSERT INTO user_preferences (user_id) VALUES ($1)")
            .bind(id)
            .execute(pool)
            .await?;
        tracing::info!(%email, "development admin created");
        id
    };

    for (address, name) in DESKS {
        let mailbox_id = sqlx::query_scalar::<_, Uuid>(
            r#"
            INSERT INTO mailboxes (address, display_name)
            VALUES ($1, $2)
            ON CONFLICT (address) DO UPDATE SET display_name = mailboxes.display_name
            RETURNING id
            "#,
        )
        .bind(address)
        .bind(name)
        .fetch_one(pool)
        .await?;
        sqlx::query("INSERT INTO mailbox_settings (mailbox_id) VALUES ($1) ON CONFLICT DO NOTHING")
            .bind(mailbox_id)
            .execute(pool)
            .await?;
        let mut connection = pool.acquire().await?;
        crate::records::ensure_system_folders(&mut connection, mailbox_id).await?;
        let is_support = *address == "support@kamfany.com";
        sqlx::query(
            r#"
            INSERT INTO mailbox_users
                (mailbox_id, user_id, can_read, can_send, can_manage, is_default)
            VALUES ($1, $2, true, true, true, false)
            ON CONFLICT (mailbox_id, user_id) DO NOTHING
            "#,
        )
        .bind(mailbox_id)
        .bind(user_id)
        .execute(pool)
        .await?;
        if is_support {
            let has_default = sqlx::query_scalar::<_, bool>(
                "SELECT EXISTS(SELECT 1 FROM mailbox_users WHERE user_id = $1 AND is_default)",
            )
            .bind(user_id)
            .fetch_one(pool)
            .await?;
            if !has_default {
                sqlx::query(
                    "UPDATE mailbox_users SET is_default = true WHERE mailbox_id = $1 AND user_id = $2",
                )
                .bind(mailbox_id)
                .bind(user_id)
                .execute(pool)
                .await?;
                sqlx::query(
                    "UPDATE user_preferences SET default_mailbox_id = $2 WHERE user_id = $1",
                )
                .bind(user_id)
                .bind(mailbox_id)
                .execute(pool)
                .await?;
            }
        }
    }
    Ok(())
}
