use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use axum::Json;
use axum::Router;
use axum::body::{Body, Bytes};
use axum::extract::{Path, State};
use axum::http::{HeaderMap, Request, StatusCode, header};
use axum::routing::{get, post};
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use hmac::{Hmac, KeyInit, Mac};
use http_body_util::BodyExt;
use kamfany_mail::auth::hash_password;
use kamfany_mail::config::AppConfig;
use kamfany_mail::delivery::{process_due, sync_provider};
use kamfany_mail::providers::ResendClient;
use kamfany_mail::router;
use kamfany_mail::state::AppState;
use serde_json::{Value, json};
use sha2::Sha256;
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

const ORIGIN: &str = "http://localhost:5173";
const SECRET_BYTES: &[u8] = b"kamfany-provider-test-secret";

#[derive(Clone)]
struct Mock {
    inbound_to: Arc<Mutex<String>>,
    inbox_address: Arc<Mutex<String>>,
}

struct Desk {
    pool: PgPool,
    app: Router,
    state: AppState,
    mock: Mock,
}

async fn desk() -> Desk {
    let _ = dotenvy::dotenv();
    let url = std::env::var("DATABASE_URL").expect("DATABASE_URL");
    let pool = sqlx::postgres::PgPoolOptions::new()
        .acquire_timeout(std::time::Duration::from_secs(5))
        .connect(&url)
        .await
        .expect("database");
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("migrations");
    let mock = Mock {
        inbound_to: Arc::new(Mutex::new(String::new())),
        inbox_address: Arc::new(Mutex::new(String::new())),
    };
    let base = mock_resend(mock.clone()).await;
    let secret_for_config = format!("whsec_{}", STANDARD.encode(SECRET_BYTES));
    let base_for_config = base.clone();
    let config = AppConfig::from_lookup(move |key| match key {
        "APP_ENV" => Some("test".to_string()),
        "CORS_ORIGINS" => Some(ORIGIN.to_string()),
        "RESEND_API_KEY" => Some("test-key".to_string()),
        "RESEND_WEBHOOK_SECRET" => Some(secret_for_config.clone()),
        "RESEND_API_BASE" => Some(base_for_config.clone()),
        "RESEND_SENDING_DOMAIN" => Some("kamfany.com".to_string()),
        _ => None,
    })
    .expect("config");
    let state = AppState::without_database(config)
        .expect("storage")
        .with_pool(pool.clone());
    let state = AppState {
        provider: Some(ResendClient::new(base, "test-key").expect("client")),
        ..state
    };
    let app = router(state.clone()).expect("router");
    Desk {
        pool,
        app,
        state,
        mock,
    }
}

async fn mock_resend(mock: Mock) -> String {
    let app = Router::new()
        .route("/emails", post(accepted_email))
        .route("/emails/receiving/{id}", get(received_email))
        .route("/inboxes", get(inboxes))
        .route("/domains", get(domains))
        .with_state(mock);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("listener");
    let address = listener.local_addr().expect("address");
    tokio::spawn(async move {
        axum::serve(listener, app).await.expect("mock resend");
    });
    format!("http://{address}")
}

async fn accepted_email(headers: HeaderMap, body: Bytes) -> Json<Value> {
    assert_eq!(
        headers
            .get(header::AUTHORIZATION)
            .and_then(|value| value.to_str().ok()),
        Some("Bearer test-key")
    );
    let id = headers
        .get("idempotency-key")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("email_sent_1");
    let payload: Value = serde_json::from_slice(&body).expect("send body");
    assert_eq!(payload["to"][0], "ada@example.com");
    assert!(
        payload["from"]
            .as_str()
            .unwrap_or_default()
            .contains("@kamfany.com")
    );
    Json(json!({ "id": id }))
}

async fn received_email(State(mock): State<Mock>, Path(id): Path<String>) -> Json<Value> {
    let to = mock.inbound_to.lock().expect("lock").clone();
    Json(json!({
        "id": id,
        "from": "Jonah Hale <jonah@example.com>",
        "to": [to],
        "cc": [],
        "subject": "Harbour invoice",
        "text": "The invoice is attached.",
        "html": "<p>The invoice is attached.</p>",
        "created_at": "2026-11-15T18:00:00Z",
        "headers": { "message-id": "<harbour@example.com>" },
        "attachments": [{ "id": "file_1", "filename": "invoice.pdf" }]
    }))
}

async fn inboxes(State(mock): State<Mock>) -> Json<Value> {
    let address = mock.inbox_address.lock().expect("lock").clone();
    Json(json!({
        "data": [{ "id": "inb_test", "email_address": address }]
    }))
}

async fn domains() -> Json<Value> {
    Json(json!({
        "data": [{
            "name": "kamfany.com",
            "status": "verified",
            "capabilities": { "sending": "enabled", "receiving": "enabled" }
        }]
    }))
}

fn sign(id: &str, timestamp: &str, body: &[u8]) -> String {
    let mut signed = Vec::new();
    signed.extend_from_slice(id.as_bytes());
    signed.push(b'.');
    signed.extend_from_slice(timestamp.as_bytes());
    signed.push(b'.');
    signed.extend_from_slice(body);
    let mut mac = <Hmac<Sha256> as KeyInit>::new_from_slice(SECRET_BYTES).expect("mac");
    mac.update(&signed);
    format!("v1,{}", STANDARD.encode(mac.finalize().into_bytes()))
}

fn unix_now() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_secs()
        .to_string()
}

async fn call(app: &Router, request: Request<Body>) -> (StatusCode, Value) {
    let response = app.clone().oneshot(request).await.expect("response");
    let status = response.status();
    let bytes = response
        .into_body()
        .collect()
        .await
        .expect("body")
        .to_bytes();
    let json = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, json)
}

async fn login_cookie(desk: &Desk, email: &str) -> String {
    let response = desk
        .app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/login")
                .header(header::ORIGIN, ORIGIN)
                .header("x-kamfany-request", "1")
                .header(
                    "x-forwarded-for",
                    format!(
                        "10.{}.{}.{}",
                        Uuid::new_v4().as_bytes()[13],
                        Uuid::new_v4().as_bytes()[14],
                        Uuid::new_v4().as_bytes()[15]
                    ),
                )
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({ "email": email, "password": "long-enough-password" }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    response
        .headers()
        .get(header::SET_COOKIE)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(';').next())
        .and_then(|value| value.strip_prefix("kamfany_session="))
        .expect("cookie")
        .to_string()
}

async fn post_webhook(desk: &Desk, id: &str, payload: &Value) {
    let body = payload.to_string();
    let timestamp = unix_now();
    let signature = sign(id, &timestamp, body.as_bytes());
    let (status, json) = call(
        &desk.app,
        Request::builder()
            .method("POST")
            .uri("/api/v1/webhooks/resend")
            .header("svix-id", id)
            .header("svix-timestamp", timestamp)
            .header("svix-signature", signature)
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(body))
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{json}");
}

#[tokio::test]
async fn sending_is_accepted_and_delivery_is_a_later_event() {
    let desk = desk().await;
    let suffix = Uuid::new_v4().simple();
    let email = format!("sender-{suffix}@kamfany.test");
    let address = format!("support-{suffix}@kamfany.com");
    let user_id = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO users (email, password_hash, display_name) VALUES ($1, $2, 'Sender') RETURNING id",
    )
    .bind(&email)
    .bind(hash_password("long-enough-password").expect("hash"))
    .fetch_one(&desk.pool)
    .await
    .expect("user");
    sqlx::query("INSERT INTO user_roles (user_id, role_id) VALUES ($1, $2)")
        .bind(user_id)
        .bind(Uuid::parse_str("10000000-0000-4000-8000-000000000003").unwrap())
        .execute(&desk.pool)
        .await
        .expect("role");
    sqlx::query("INSERT INTO user_preferences (user_id) VALUES ($1)")
        .bind(user_id)
        .execute(&desk.pool)
        .await
        .expect("preferences");
    let mailbox_id = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO mailboxes (address, display_name, is_enabled) VALUES ($1, 'KAMFANY Support', true) RETURNING id",
    )
    .bind(&address)
    .fetch_one(&desk.pool)
    .await
    .expect("mailbox");
    sqlx::query(
        "INSERT INTO mailbox_users (mailbox_id, user_id, can_read, can_send, can_manage) VALUES ($1, $2, true, true, false)",
    )
    .bind(mailbox_id)
    .bind(user_id)
    .execute(&desk.pool)
    .await
    .expect("grant");

    let token = login_cookie(&desk, &email).await;
    let (status, json) = call(
        &desk.app,
        Request::builder()
            .method("POST")
            .uri(format!("/api/v1/mailboxes/{mailbox_id}/send"))
            .header(header::ORIGIN, ORIGIN)
            .header("x-kamfany-request", "1")
            .header(header::COOKIE, format!("kamfany_session={token}"))
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(
                json!({
                    "to": ["ada@example.com"],
                    "subject": "Harbour",
                    "text": "Please find the note."
                })
                .to_string(),
            ))
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{json}");
    assert_eq!(json["data"]["status"], "accepted");
    let provider_id = json["data"]["provider_message_id"]
        .as_str()
        .unwrap()
        .to_string();

    post_webhook(
        &desk,
        &format!("msg_delivered_{suffix}"),
        &json!({ "type": "email.delivered", "data": { "email_id": provider_id } }),
    )
    .await;
    process_due(&desk.state, 5).await.expect("jobs");
    let stored: String = sqlx::query_scalar(
        "SELECT status FROM messages WHERE provider = 'resend' AND provider_message_id = $2 AND mailbox_id = $1",
    )
    .bind(mailbox_id)
    .bind(&provider_id)
    .fetch_one(&desk.pool)
    .await
    .unwrap();
    assert_eq!(stored, "delivered");

    *desk.mock.inbound_to.lock().expect("lock") = address.clone();
    post_webhook(
        &desk,
        &format!("msg_received_{suffix}"),
        &json!({
            "type": "email.received",
            "data": { "email_id": format!("email_in_{suffix}"), "to": [address] }
        }),
    )
    .await;
    process_due(&desk.state, 5).await.expect("inbound");
    let inbound: (String, bool) = sqlx::query_as(
        r#"
        SELECT m.subject, mb.is_receiving_verified
        FROM messages m
        JOIN mailboxes mb ON mb.id = m.mailbox_id
        WHERE m.provider_message_id = $2 AND m.mailbox_id = $1
        "#,
    )
    .bind(mailbox_id)
    .bind(format!("email_in_{suffix}"))
    .fetch_one(&desk.pool)
    .await
    .expect("inbound message");
    assert_eq!(inbound.0, "Harbour invoice");
    assert!(inbound.1);

    let (status, _) = call(
        &desk.app,
        Request::builder()
            .method("POST")
            .uri("/api/v1/webhooks/resend")
            .body(Body::from("{}"))
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    sqlx::query("DELETE FROM messages WHERE mailbox_id = $1")
        .bind(mailbox_id)
        .execute(&desk.pool)
        .await
        .ok();
    sqlx::query("DELETE FROM mailboxes WHERE id = $1")
        .bind(mailbox_id)
        .execute(&desk.pool)
        .await
        .ok();
    sqlx::query("DELETE FROM users WHERE id = $1")
        .bind(user_id)
        .execute(&desk.pool)
        .await
        .ok();
}

#[tokio::test]
async fn provider_sync_marks_only_a_listed_inbox_verified() {
    let desk = desk().await;
    let suffix = Uuid::new_v4().simple();
    let listed = format!("listed-{suffix}@kamfany.com");
    let other = format!("other-{suffix}@kamfany.com");
    *desk.mock.inbox_address.lock().expect("lock") = listed.clone();
    let listed_id = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO mailboxes (address, display_name) VALUES ($1, 'Listed') RETURNING id",
    )
    .bind(&listed)
    .fetch_one(&desk.pool)
    .await
    .unwrap();
    let other_id = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO mailboxes (address, display_name) VALUES ($1, 'Other') RETURNING id",
    )
    .bind(&other)
    .fetch_one(&desk.pool)
    .await
    .unwrap();

    let report = sync_provider(&desk.state).await.expect("sync");
    assert_eq!(report.domain_status, "verified");
    assert_eq!(report.receiving, "enabled");
    assert!(
        report
            .verified_addresses
            .iter()
            .any(|address| address == &listed)
    );
    let listed_verified: bool =
        sqlx::query_scalar("SELECT is_receiving_verified FROM mailboxes WHERE id = $1")
            .bind(listed_id)
            .fetch_one(&desk.pool)
            .await
            .unwrap();
    let other_verified: bool =
        sqlx::query_scalar("SELECT is_receiving_verified FROM mailboxes WHERE id = $1")
            .bind(other_id)
            .fetch_one(&desk.pool)
            .await
            .unwrap();
    assert!(listed_verified);
    assert!(!other_verified);

    sqlx::query("DELETE FROM mailboxes WHERE id IN ($1, $2)")
        .bind(listed_id)
        .bind(other_id)
        .execute(&desk.pool)
        .await
        .ok();
}
