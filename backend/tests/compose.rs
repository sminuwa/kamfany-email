use std::time::Duration;

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
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
        "STORAGE_DIR" => Some(
            std::env::temp_dir()
                .join("kamfany-mail-compose-test")
                .display()
                .to_string(),
        ),
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

async fn login(desk: &Desk, email: &str) -> String {
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

async fn call(
    app: &axum::Router,
    request: Request<Body>,
) -> (StatusCode, serde_json::Value, Vec<u8>) {
    let response = app.clone().oneshot(request).await.expect("response");
    let status = response.status();
    let bytes = response
        .into_body()
        .collect()
        .await
        .expect("body")
        .to_bytes();
    let json = serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null);
    (status, json, bytes.to_vec())
}

#[tokio::test]
async fn a_draft_keeps_its_file_until_it_is_sent_or_discarded() {
    let desk = desk().await;
    let suffix = Uuid::new_v4().simple();
    let owner_email = format!("composer-{suffix}@kamfany.test");
    let stranger_email = format!("stranger-{suffix}@kamfany.test");
    let owner = create_user(
        &desk.pool,
        &owner_email,
        "10000000-0000-4000-8000-000000000001",
    )
    .await;
    let stranger = create_user(
        &desk.pool,
        &stranger_email,
        "10000000-0000-4000-8000-000000000003",
    )
    .await;
    let mailbox = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO mailboxes (address, display_name, is_enabled) VALUES ($1, 'Desk', true) RETURNING id",
    )
    .bind(format!("desk-{suffix}@kamfany.com"))
    .fetch_one(&desk.pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO mailbox_users (mailbox_id, user_id, can_read, can_send, can_manage) VALUES ($1, $2, true, true, false)",
    )
    .bind(mailbox)
    .bind(owner)
    .execute(&desk.pool)
    .await
    .unwrap();

    let cookie = login(&desk, &owner_email).await;
    let (status, json, _) = call(
        &desk.app,
        Request::builder()
            .method("POST")
            .uri(format!("/api/v1/mailboxes/{mailbox}/drafts"))
            .header(header::ORIGIN, ORIGIN)
            .header("x-kamfany-request", "1")
            .header(header::COOKIE, format!("kamfany_session={cookie}"))
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(
                json!({
                    "kind": "new",
                    "to": ["ada@example.com"],
                    "subject": "Harbour note",
                    "html": "<p>Hello <strong>there</strong></p><script>no</script>"
                })
                .to_string(),
            ))
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{json}");
    assert!(
        !json["data"]["body_html"]
            .as_str()
            .unwrap()
            .contains("script")
    );
    let draft_id = json["data"]["id"].as_str().unwrap();

    let boundary = "kamfanyboundary";
    let body = format!(
        "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"note.txt\"\r\nContent-Type: text/plain\r\n\r\nhello desk\r\n--{boundary}--\r\n"
    );
    let (status, json, _) = call(
        &desk.app,
        Request::builder()
            .method("POST")
            .uri(format!("/api/v1/drafts/{draft_id}/attachments"))
            .header(header::ORIGIN, ORIGIN)
            .header("x-kamfany-request", "1")
            .header(header::COOKIE, format!("kamfany_session={cookie}"))
            .header(
                header::CONTENT_TYPE,
                format!("multipart/form-data; boundary={boundary}"),
            )
            .body(Body::from(body))
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{json}");
    assert_eq!(json["data"]["scan_status"], "pending");
    let file_id = json["data"]["id"].as_str().unwrap();

    let (status, _, bytes) = call(
        &desk.app,
        Request::builder()
            .method("GET")
            .uri(format!("/api/v1/attachments/{file_id}"))
            .header(header::ORIGIN, ORIGIN)
            .header("x-kamfany-request", "1")
            .header(header::COOKIE, format!("kamfany_session={cookie}"))
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(bytes, b"hello desk");

    let stranger_cookie = login(&desk, &stranger_email).await;
    let (status, _, _) = call(
        &desk.app,
        Request::builder()
            .method("GET")
            .uri(format!("/api/v1/attachments/{file_id}"))
            .header(header::ORIGIN, ORIGIN)
            .header("x-kamfany-request", "1")
            .header(header::COOKIE, format!("kamfany_session={stranger_cookie}"))
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    let (status, json, _) = call(
        &desk.app,
        Request::builder()
            .method("GET")
            .uri(format!("/api/v1/mailboxes/{mailbox}/messages?view=drafts"))
            .header(header::ORIGIN, ORIGIN)
            .header("x-kamfany-request", "1")
            .header(header::COOKIE, format!("kamfany_session={cookie}"))
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{json}");
    assert_eq!(json["data"]["messages"][0]["subject"], "Harbour note");
    assert_eq!(json["data"]["messages"][0]["status"], "draft");

    let (status, json, _) = call(
        &desk.app,
        Request::builder()
            .method("POST")
            .uri(format!("/api/v1/mailboxes/{mailbox}/send"))
            .header(header::ORIGIN, ORIGIN)
            .header("x-kamfany-request", "1")
            .header(header::COOKIE, format!("kamfany_session={cookie}"))
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(
                json!({
                    "to": ["ada@example.com"],
                    "subject": "Harbour note",
                    "html": "<p>Hello there</p>",
                    "draft_id": draft_id
                })
                .to_string(),
            ))
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE, "{json}");
    let remaining: i64 = sqlx::query_scalar("SELECT count(*) FROM drafts WHERE id = $1")
        .bind(Uuid::parse_str(draft_id).unwrap())
        .fetch_one(&desk.pool)
        .await
        .unwrap();
    assert_eq!(remaining, 1);

    let (status, _, _) = call(
        &desk.app,
        Request::builder()
            .method("DELETE")
            .uri(format!("/api/v1/drafts/{draft_id}"))
            .header(header::ORIGIN, ORIGIN)
            .header("x-kamfany-request", "1")
            .header(header::COOKIE, format!("kamfany_session={cookie}"))
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    sqlx::query("DELETE FROM messages WHERE mailbox_id = $1")
        .bind(mailbox)
        .execute(&desk.pool)
        .await
        .ok();
    sqlx::query("DELETE FROM mailboxes WHERE id = $1")
        .bind(mailbox)
        .execute(&desk.pool)
        .await
        .ok();
    sqlx::query("DELETE FROM users WHERE id = ANY($1)")
        .bind(vec![owner, stranger])
        .execute(&desk.pool)
        .await
        .ok();
}

async fn create_user(pool: &PgPool, email: &str, role: &str) -> Uuid {
    let id = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO users (email, password_hash, display_name) VALUES ($1, $2, 'Composer') RETURNING id",
    )
    .bind(email)
    .bind(hash_password("long-enough-password").unwrap())
    .fetch_one(pool)
    .await
    .unwrap();
    sqlx::query("INSERT INTO user_roles (user_id, role_id) VALUES ($1, $2)")
        .bind(id)
        .bind(Uuid::parse_str(role).unwrap())
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO user_preferences (user_id) VALUES ($1)")
        .bind(id)
        .execute(pool)
        .await
        .unwrap();
    id
}
