use std::time::Duration;

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use kamfany_mail::auth::hash_password;
use kamfany_mail::config::AppConfig;
use kamfany_mail::delivery::{GroupDelivery, deliver_group_mail};
use kamfany_mail::groups::fanout;
use kamfany_mail::router;
use kamfany_mail::state::AppState;
use serde_json::json;
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

const ORIGIN: &str = "http://localhost:5173";

async fn desk() -> (PgPool, axum::Router) {
    let _ = dotenvy::dotenv();
    let url = std::env::var("DATABASE_URL").expect("DATABASE_URL");
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .acquire_timeout(Duration::from_secs(5))
        .connect(&url)
        .await
        .expect("database");
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("migrations");
    let config = AppConfig::from_lookup(|key| match key {
        "APP_ENV" => Some("test".to_string()),
        "CORS_ORIGINS" => Some(ORIGIN.to_string()),
        _ => None,
    })
    .expect("config");
    let app = router(
        AppState::without_database(config)
            .expect("storage")
            .with_pool(pool.clone()),
    )
    .expect("router");
    (pool, app)
}

async fn create_user(pool: &PgPool, email: &str, role: Uuid) -> Uuid {
    let id = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO users (email, password_hash, display_name) VALUES ($1, $2, 'Test Person') RETURNING id",
    )
    .bind(email)
    .bind(hash_password("long-enough-password").expect("hash"))
    .fetch_one(pool)
    .await
    .expect("user");
    sqlx::query("INSERT INTO user_roles (user_id, role_id) VALUES ($1, $2)")
        .bind(id)
        .bind(role)
        .execute(pool)
        .await
        .expect("role");
    sqlx::query("INSERT INTO user_preferences (user_id) VALUES ($1)")
        .bind(id)
        .execute(pool)
        .await
        .expect("preferences");
    let mailbox_id = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO mailboxes (address, display_name, is_enabled) VALUES ($1, 'Test Person', true) RETURNING id",
    )
    .bind(email)
    .fetch_one(pool)
    .await
    .expect("mailbox");
    sqlx::query("INSERT INTO mailbox_settings (mailbox_id) VALUES ($1)")
        .bind(mailbox_id)
        .execute(pool)
        .await
        .expect("settings");
    sqlx::query(
        "INSERT INTO mailbox_users (mailbox_id, user_id, can_read, can_send, can_manage, is_default) VALUES ($1, $2, true, true, true, true)",
    )
    .bind(mailbox_id)
    .bind(id)
    .execute(pool)
    .await
    .expect("grant");
    mailbox_id
}

async fn call(
    app: &axum::Router,
    method: &str,
    uri: &str,
    cookie: Option<&str>,
    body: Option<serde_json::Value>,
) -> (StatusCode, serde_json::Value, Option<String>) {
    let mut builder = Request::builder()
        .method(method)
        .uri(uri)
        .header(header::ORIGIN, ORIGIN)
        .header("x-kamfany-request", "1");
    if let Some(cookie) = cookie {
        builder = builder.header(header::COOKIE, format!("kamfany_session={cookie}"));
    }
    if body.is_some() {
        builder = builder.header(header::CONTENT_TYPE, "application/json");
    }
    let request = builder
        .body(Body::from(
            body.map(|value| value.to_string()).unwrap_or_default(),
        ))
        .expect("request");
    let response = app.clone().oneshot(request).await.expect("response");
    let status = response.status();
    let cookie = response
        .headers()
        .get(header::SET_COOKIE)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(';').next())
        .and_then(|value| value.strip_prefix("kamfany_session="))
        .map(str::to_string);
    let bytes = response.into_body().collect().await.expect("body").to_bytes();
    let json = if bytes.is_empty() {
        serde_json::Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null)
    };
    (status, json, cookie)
}

#[tokio::test]
async fn the_owner_creates_a_group_and_mail_reaches_each_mailbox() {
    let (pool, app) = desk().await;
    let suffix = Uuid::new_v4().simple();
    let owner_role = uuid::uuid!("10000000-0000-4000-8000-000000000001");
    let member_role = uuid::uuid!("10000000-0000-4000-8000-000000000003");
    let owner_mailbox = create_user(&pool, &format!("owner-{suffix}@kamfany.test"), owner_role).await;
    let first = create_user(&pool, &format!("ada-{suffix}@kamfany.test"), member_role).await;
    let second = create_user(&pool, &format!("bola-{suffix}@kamfany.test"), member_role).await;
    let local = format!("team-{suffix}");

    let (status, _, owner_cookie) = call(
        &app,
        "POST",
        "/api/v1/auth/login",
        None,
        Some(json!({
            "email": format!("owner-{suffix}@kamfany.test"),
            "password": "long-enough-password"
        })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let owner_cookie = owner_cookie.expect("session");

    let (status, _, member_cookie) = call(
        &app,
        "POST",
        "/api/v1/auth/login",
        None,
        Some(json!({
            "email": format!("ada-{suffix}@kamfany.test"),
            "password": "long-enough-password"
        })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let member_cookie = member_cookie.expect("member session");

    let (status, _, _) = call(
        &app,
        "POST",
        "/api/v1/groups",
        Some(&member_cookie),
        Some(json!({
            "name": "Team",
            "address": local,
            "mailbox_ids": [first, second]
        })),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    let (status, created, _) = call(
        &app,
        "POST",
        "/api/v1/groups",
        Some(&owner_cookie),
        Some(json!({
            "name": "Team",
            "address": local,
            "mailbox_ids": [first, second, owner_mailbox]
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let address = created["data"]["address"].as_str().expect("address").to_string();
    assert!(address.starts_with(&format!("team-{suffix}@")));
    assert_eq!(created["data"]["mailboxes"].as_array().expect("members").len(), 3);

    let (status, listed, _) = call(&app, "GET", "/api/v1/groups", Some(&member_cookie), None).await;
    assert_eq!(status, StatusCode::OK);
    assert!(
        listed["data"]
            .as_array()
            .expect("groups")
            .iter()
            .any(|group| group["address"] == address)
    );

    let expanded = fanout(&pool, std::slice::from_ref(&address), &[], &[])
        .await
        .expect("fanout");
    assert_eq!(expanded.to.len(), 3);
    assert_eq!(expanded.mailboxes.len(), 3);

    let state = AppState::without_database(
        AppConfig::from_lookup(|key| match key {
            "APP_ENV" => Some("test".to_string()),
            "CORS_ORIGINS" => Some(ORIGIN.to_string()),
            _ => None,
        })
        .expect("config"),
    )
    .expect("storage")
    .with_pool(pool.clone());
    let from_address = format!("owner-{suffix}@kamfany.test");
    let provider_message_id = format!("provider-{suffix}");
    deliver_group_mail(
        &state,
        &GroupDelivery {
            from_address: &from_address,
            from_name: "Owner",
            subject: "Hello team",
            text: Some("The note"),
            html: None,
            to: std::slice::from_ref(&address),
            cc: &[],
            message_id_header: "<group-test@kamfany.com>",
            provider_message_id: &provider_message_id,
            mailbox_ids: &expanded.mailboxes,
            files: &[],
        },
    )
    .await
    .expect("deliver");

    let copies = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT count(*)
        FROM messages m
        JOIN message_folder_assignments a ON a.message_id = m.id
        JOIN folders f ON f.id = a.folder_id
        WHERE m.subject = 'Hello team'
          AND m.direction = 'inbound'
          AND f.system_key = 'inbox'
          AND m.mailbox_id = ANY($1)
        "#,
    )
    .bind(&expanded.mailboxes)
    .fetch_one(&pool)
    .await
    .expect("count");
    assert_eq!(copies, 3);
}
