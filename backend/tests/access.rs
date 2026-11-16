use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::Json;
use axum::Router;
use axum::body::{Body, Bytes};
use axum::extract::State;
use axum::http::{Request, StatusCode, header};
use axum::routing::post;
use http_body_util::BodyExt;
use kamfany_mail::auth::hash_password;
use kamfany_mail::config::AppConfig;
use kamfany_mail::router;
use kamfany_mail::state::AppState;
use serde_json::json;
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

const ORIGIN: &str = "http://localhost:5173";

struct Desk {
    pool: PgPool,
    app: axum::Router,
}

async fn desk() -> Desk {
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
    Desk { pool, app }
}

async fn create_user(pool: &PgPool, email: &str, password: &str, role: Uuid) -> Uuid {
    let id = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO users (email, password_hash, display_name) VALUES ($1, $2, 'Test Person') RETURNING id",
    )
    .bind(email)
    .bind(hash_password(password).expect("hash"))
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
    id
}

fn call(
    method: &str,
    uri: &str,
    cookie: Option<&str>,
    body: Option<serde_json::Value>,
    browser: bool,
) -> Request<Body> {
    let mut builder = Request::builder().method(method).uri(uri);
    if browser {
        let bytes = *Uuid::new_v4().as_bytes();
        builder = builder
            .header(header::ORIGIN, ORIGIN)
            .header("x-kamfany-request", "1")
            .header(
                "x-forwarded-for",
                format!("10.{}.{}.{}", bytes[13], bytes[14], bytes[15]),
            );
    }
    if let Some(cookie) = cookie {
        builder = builder.header(header::COOKIE, format!("kamfany_session={cookie}"));
    }
    if body.is_some() {
        builder = builder.header(header::CONTENT_TYPE, "application/json");
    }
    builder
        .body(Body::from(
            body.map(|value| value.to_string()).unwrap_or_default(),
        ))
        .expect("request")
}

async fn send(
    app: &axum::Router,
    request: Request<Body>,
) -> (StatusCode, serde_json::Value, Option<String>) {
    let response = app.clone().oneshot(request).await.expect("response");
    let status = response.status();
    let cookie = response
        .headers()
        .get(header::SET_COOKIE)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(';').next())
        .and_then(|value| value.strip_prefix("kamfany_session="))
        .map(str::to_string);
    let bytes = response
        .into_body()
        .collect()
        .await
        .expect("body")
        .to_bytes();
    let json = if bytes.is_empty() {
        serde_json::Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null)
    };
    (status, json, cookie)
}

fn session_token(cookie: &Option<String>) -> &str {
    cookie.as_deref().expect("session cookie")
}

#[tokio::test]
async fn login_without_a_database_is_not_ready() {
    let config = AppConfig::from_lookup(|_| None).expect("defaults");
    let app = router(AppState::without_database(config).expect("storage")).expect("router");
    let (status, json, _) = send(
        &app,
        call(
            "POST",
            "/api/v1/auth/login",
            None,
            Some(json!({ "email": "adaeze@kamfany.com", "password": "long-enough-password" })),
            true,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(json["error"]["code"], "not_ready");
}

#[tokio::test]
async fn a_cross_site_login_is_blocked() {
    let config = AppConfig::from_lookup(|_| None).expect("defaults");
    let app = router(AppState::without_database(config).expect("storage")).expect("router");
    let (status, json, _) = send(
        &app,
        call(
            "POST",
            "/api/v1/auth/login",
            None,
            Some(json!({ "email": "adaeze@kamfany.com", "password": "long-enough-password" })),
            false,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(json["error"]["code"], "forbidden");
}

#[tokio::test]
async fn sessions_and_mailbox_grants_are_enforced() {
    let desk = desk().await;
    let suffix = Uuid::new_v4().simple();
    let owner_email = format!("owner-{suffix}@kamfany.test");
    let member_email = format!("member-{suffix}@kamfany.test");
    let password = "long-enough-password";
    let owner_role = uuid::uuid!("10000000-0000-4000-8000-000000000001");
    let member_role = uuid::uuid!("10000000-0000-4000-8000-000000000003");
    let owner_id = create_user(&desk.pool, &owner_email, password, owner_role).await;
    let member_id = create_user(&desk.pool, &member_email, password, member_role).await;

    let (status, json, _) = send(
        &desk.app,
        call(
            "POST",
            "/api/v1/auth/login",
            None,
            Some(json!({ "email": owner_email, "password": "wrong-password-value" })),
            true,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(json["error"]["message"], "Email or password is incorrect.");

    let (status, json, _) = send(
        &desk.app,
        call(
            "POST",
            "/api/v1/auth/login",
            None,
            Some(
                json!({ "email": format!("missing-{suffix}@kamfany.test"), "password": password }),
            ),
            true,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(json["error"]["message"], "Email or password is incorrect.");

    let (status, owner_body, owner_cookie) = send(
        &desk.app,
        call(
            "POST",
            "/api/v1/auth/login",
            None,
            Some(json!({ "email": owner_email, "password": password })),
            true,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(owner_body["data"]["email"], owner_email);
    assert!(
        owner_body["data"]["permissions"]
            .as_array()
            .expect("permissions")
            .iter()
            .any(|item| item == "mailbox.manage")
    );
    let owner_token = session_token(&owner_cookie).to_string();

    let (status, _, _) = send(&desk.app, call("GET", "/api/v1/auth/me", None, None, true)).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    let mailbox_id = sqlx::query_scalar::<_, Uuid>("SELECT id FROM mailboxes WHERE address = $1")
        .bind(&owner_email)
        .fetch_one(&desk.pool)
        .await
        .expect("mailbox")
        .to_string();
    let (status, _, _) = send(
        &desk.app,
        call(
            "POST",
            "/api/v1/mailboxes",
            Some(&owner_token),
            Some(json!({ "address": format!("other-{suffix}@kamfany.test"), "display_name": "Other" })),
            true,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);

    let (status, patched, _) = send(
        &desk.app,
        call(
            "PATCH",
            &format!("/api/v1/mailboxes/{mailbox_id}"),
            Some(&owner_token),
            Some(json!({ "is_enabled": true, "is_receiving_verified": true, "display_name": "Company desk" })),
            true,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(patched["data"]["is_enabled"], true);
    assert_eq!(patched["data"]["is_receiving_verified"], false);
    assert_eq!(patched["data"]["display_name"], "Company desk");

    let (status, member_body, member_cookie) = send(
        &desk.app,
        call(
            "POST",
            "/api/v1/auth/login",
            None,
            Some(json!({ "email": member_email, "password": password })),
            true,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let member_token = session_token(&member_cookie).to_string();
    assert_eq!(member_body["data"]["roles"][0], "member");

    let (status, listed, _) = send(
        &desk.app,
        call("GET", "/api/v1/mailboxes", Some(&member_token), None, true),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(listed["data"].as_array().expect("list").len(), 1);
    assert_eq!(listed["data"][0]["address"], member_email);

    let (status, _, _) = send(
        &desk.app,
        call(
            "GET",
            &format!("/api/v1/mailboxes/{mailbox_id}"),
            Some(&member_token),
            None,
            true,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    let (status, _, _) = send(
        &desk.app,
        call(
            "POST",
            "/api/v1/mailboxes",
            Some(&member_token),
            Some(json!({ "address": format!("other-{suffix}@kamfany.test"), "display_name": "Other" })),
            true,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    let (status, _, _) = send(
        &desk.app,
        call(
            "POST",
            &format!("/api/v1/mailboxes/{mailbox_id}/members"),
            Some(&owner_token),
            Some(json!({ "email": member_email, "can_read": true, "can_send": false, "can_manage": false })),
            true,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);

    let (status, granted, _) = send(
        &desk.app,
        call("GET", "/api/v1/mailboxes", Some(&member_token), None, true),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(granted["data"][0]["address"], member_email);

    let (status, _, rotated) = send(
        &desk.app,
        call(
            "POST",
            "/api/v1/auth/refresh",
            Some(&owner_token),
            None,
            true,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let rotated_token = session_token(&rotated).to_string();
    assert_ne!(rotated_token, owner_token);

    let (status, _, _) = send(
        &desk.app,
        call("GET", "/api/v1/auth/me", Some(&owner_token), None, true),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    let (status, _, _) = send(
        &desk.app,
        call(
            "POST",
            "/api/v1/auth/logout",
            Some(&rotated_token),
            None,
            true,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (status, _, _) = send(
        &desk.app,
        call("GET", "/api/v1/auth/me", Some(&rotated_token), None, true),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    sqlx::query("DELETE FROM mailboxes WHERE id = $1")
        .bind(Uuid::parse_str(&mailbox_id).expect("uuid"))
        .execute(&desk.pool)
        .await
        .expect("delete mailbox");
    sqlx::query("DELETE FROM users WHERE id = $1 OR id = $2")
        .bind(owner_id)
        .bind(member_id)
        .execute(&desk.pool)
        .await
        .expect("delete users");
}

#[tokio::test]
async fn password_reset_does_not_pretend_to_send_mail() {
    let config = AppConfig::from_lookup(|_| None).expect("defaults");
    let app = router(AppState::without_database(config).expect("storage")).expect("router");
    let (status, json, _) = send(
        &app,
        call(
            "POST",
            "/api/v1/auth/forgot-password",
            None,
            Some(json!({ "email": "adaeze@kamfany.com" })),
            true,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(
        json["data"]["message"]
            .as_str()
            .expect("message")
            .contains("mail delivery is connected")
    );
    let (status, json, _) = send(
        &app,
        call(
            "POST",
            "/api/v1/auth/reset-password",
            None,
            Some(json!({ "token": "nope", "password": "long-enough-password" })),
            true,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(json["error"]["code"], "not_ready");
}

#[tokio::test]
async fn a_reset_link_replaces_the_password_once() {
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
    let sent = Arc::new(Mutex::new(Vec::new()));
    let base = capture_mail(sent.clone()).await;
    let base_for_config = base.clone();
    let config = AppConfig::from_lookup(move |key| match key {
        "APP_ENV" => Some("test".to_string()),
        "CORS_ORIGINS" => Some(ORIGIN.to_string()),
        "RESEND_API_KEY" => Some("test-key".to_string()),
        "RESEND_API_BASE" => Some(base_for_config.clone()),
        "RESEND_SENDING_DOMAIN" => Some("kamfany.com".to_string()),
        _ => None,
    })
    .expect("config");
    let app = router(
        AppState::without_database(config)
            .expect("storage")
            .with_pool(pool.clone()),
    )
    .expect("router");
    let owner_role = uuid::uuid!("10000000-0000-4000-8000-000000000001");
    let email = format!("reset-{}@example.com", Uuid::new_v4().simple());
    let user_id = create_user(&pool, &email, "long-enough-password", owner_role).await;

    let (status, json, _) = send(
        &app,
        call(
            "POST",
            "/api/v1/auth/forgot-password",
            None,
            Some(json!({ "email": email })),
            true,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(
        json["data"]["message"]
            .as_str()
            .unwrap()
            .contains("on its way")
    );
    let body = sent.lock().expect("sent mail").pop().expect("reset mail");
    let token = body
        .split("token=")
        .nth(1)
        .and_then(|value| value.split(|ch: char| !ch.is_ascii_hexdigit()).next())
        .expect("token");
    assert_eq!(token.len(), 64);

    let (status, missing, _) = send(
        &app,
        call(
            "POST",
            "/api/v1/auth/forgot-password",
            None,
            Some(json!({ "email": "missing-person@example.com" })),
            true,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(sent.lock().expect("sent mail").is_empty());
    assert!(
        missing["data"]["message"]
            .as_str()
            .unwrap()
            .contains("on its way")
    );

    let (status, _, _) = send(
        &app,
        call(
            "POST",
            "/api/v1/auth/reset-password",
            None,
            Some(json!({ "token": token, "password": "a-different-password" })),
            true,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, _, cookie) = send(
        &app,
        call(
            "POST",
            "/api/v1/auth/login",
            None,
            Some(json!({ "email": email, "password": "a-different-password" })),
            true,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(cookie.is_some());
    let (status, _, _) = send(
        &app,
        call(
            "POST",
            "/api/v1/auth/login",
            None,
            Some(json!({ "email": email, "password": "long-enough-password" })),
            true,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let (status, _, _) = send(
        &app,
        call(
            "POST",
            "/api/v1/auth/reset-password",
            None,
            Some(json!({ "token": token, "password": "yet-another-password" })),
            true,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);

    sqlx::query("DELETE FROM users WHERE id = $1")
        .bind(user_id)
        .execute(&pool)
        .await
        .expect("delete user");
}

async fn capture_mail(sent: Arc<Mutex<Vec<String>>>) -> String {
    let app = Router::new()
        .route("/emails", post(store_mail))
        .with_state(sent);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("listener");
    let address = listener.local_addr().expect("address");
    tokio::spawn(async move {
        axum::serve(listener, app).await.expect("mock resend");
    });
    format!("http://{address}")
}

async fn store_mail(
    State(sent): State<Arc<Mutex<Vec<String>>>>,
    body: Bytes,
) -> Json<serde_json::Value> {
    sent.lock()
        .expect("sent mail")
        .push(String::from_utf8_lossy(&body).into_owned());
    Json(json!({ "id": "email_reset_1" }))
}
