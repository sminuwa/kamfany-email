use std::time::Duration;

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use chrono::Utc;
use http_body_util::BodyExt;
use kamfany_mail::auth::hash_password;
use kamfany_mail::config::AppConfig;
use kamfany_mail::delivery::store_inbound;
use kamfany_mail::live::announce;
use kamfany_mail::providers::InboundEmail;
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
    state: AppState,
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
                .join("kamfany-mail-productivity-test")
                .display()
                .to_string(),
        ),
        _ => None,
    })
    .expect("config");
    let state = AppState::without_database(config)
        .expect("storage")
        .with_pool(pool.clone());
    let app = router(state.clone()).expect("router");
    Desk { pool, app, state }
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

async fn call(app: &axum::Router, request: Request<Body>) -> (StatusCode, serde_json::Value) {
    let response = app.clone().oneshot(request).await.expect("response");
    let status = response.status();
    let bytes = response.into_body().collect().await.expect("body").to_bytes();
    let json = serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null);
    (status, json)
}

fn authed(method: &str, uri: &str, cookie: &str, body: Option<String>) -> Request<Body> {
    let mut builder = Request::builder()
        .method(method)
        .uri(uri)
        .header(header::ORIGIN, ORIGIN)
        .header("x-kamfany-request", "1")
        .header(header::COOKIE, format!("kamfany_session={cookie}"));
    if body.is_some() {
        builder = builder.header(header::CONTENT_TYPE, "application/json");
    }
    builder.body(Body::from(body.unwrap_or_default())).unwrap()
}

#[tokio::test]
async fn contacts_search_and_notices_stay_with_their_owner() {
    let desk = desk().await;
    let suffix = Uuid::new_v4().simple();
    let owner_email = format!("reader-{suffix}@kamfany.test");
    let stranger_email = format!("outsider-{suffix}@kamfany.test");
    let owner = create_user(&desk.pool, &owner_email).await;
    let stranger = create_user(&desk.pool, &stranger_email).await;
    let mailbox = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO mailboxes (address, display_name, is_enabled) VALUES ($1, 'Desk', true) RETURNING id",
    )
    .bind(format!("desk-{suffix}@kamfany.com"))
    .fetch_one(&desk.pool)
    .await
    .unwrap();
    let hidden = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO mailboxes (address, display_name, is_enabled) VALUES ($1, 'Other', true) RETURNING id",
    )
    .bind(format!("other-{suffix}@kamfany.com"))
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
    sqlx::query(
        "INSERT INTO mailbox_users (mailbox_id, user_id, can_read, can_send, can_manage) VALUES ($1, $2, true, false, false)",
    )
    .bind(hidden)
    .bind(stranger)
    .execute(&desk.pool)
    .await
    .unwrap();

    let owner_cookie = login(&desk, &owner_email).await;
    let stranger_cookie = login(&desk, &stranger_email).await;

    let (status, created) = call(
        &desk.app,
        authed(
            "POST",
            "/api/v1/contacts",
            &owner_cookie,
            Some(json!({ "full_name": "Adaeze Okonkwo", "email": "adaeze@example.com", "company": "IT KAMFANY" }).to_string()),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{created}");
    let contact_id = created["data"]["id"].as_str().unwrap();

    let (status, page) = call(
        &desk.app,
        authed("GET", "/api/v1/contacts?q=adaeze", &owner_cookie, None),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{page}");
    assert_eq!(page["data"][0]["full_name"], "Adaeze Okonkwo");

    let (status, hidden_list) = call(
        &desk.app,
        authed("GET", "/api/v1/contacts?q=adaeze", &stranger_cookie, None),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{hidden_list}");
    assert!(hidden_list["data"].as_array().unwrap().is_empty());
    let (status, missing) = call(
        &desk.app,
        authed(
            "PATCH",
            &format!("/api/v1/contacts/{contact_id}"),
            &stranger_cookie,
            Some(json!({ "full_name": "Nope" }).to_string()),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{missing}");

    let boundary = "kamfany-bound";
    let csv = "name,email,company\nHarbour Clerk,clerk@example.com,Northwind\n";
    let body = format!(
        "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"people.csv\"\r\nContent-Type: text/csv\r\n\r\n{csv}\r\n--{boundary}--\r\n"
    );
    let response = desk
        .app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/contacts/import")
                .header(header::ORIGIN, ORIGIN)
                .header("x-kamfany-request", "1")
                .header(header::COOKIE, format!("kamfany_session={owner_cookie}"))
                .header(header::CONTENT_TYPE, format!("multipart/form-data; boundary={boundary}"))
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let address = format!("desk-{suffix}@kamfany.com");
    let other = format!("other-{suffix}@kamfany.com");
    let stored = store_inbound(
        &desk.pool,
        &mail(&address, &format!("in-{suffix}"), "Harbour invoice"),
        &[],
    )
    .await
    .expect("stored");
    assert_eq!(stored.len(), 1);
    store_inbound(
        &desk.pool,
        &mail(&other, &format!("hidden-{suffix}"), "Private ledger"),
        &[],
    )
    .await
    .expect("hidden mail");

    let (status, found) = call(
        &desk.app,
        authed(
            "GET",
            &format!("/api/v1/search?q=harbour&mailbox_id={mailbox}"),
            &owner_cookie,
            None,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{found}");
    assert_eq!(found["data"]["messages"][0]["subject"], "Harbour invoice");
    assert_eq!(found["data"]["messages"].as_array().unwrap().len(), 1);

    let (status, missed) = call(
        &desk.app,
        authed("GET", "/api/v1/search?q=ledger", &owner_cookie, None),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{missed}");
    assert!(missed["data"]["messages"].as_array().unwrap().is_empty());

    announce(&desk.state, &stored[0]).await.expect("notice");
    announce(&desk.state, &stored[0]).await.expect("again");
    let notices: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM notifications WHERE user_id = $1 AND message_id = $2",
    )
    .bind(owner)
    .bind(stored[0].message_id)
    .fetch_one(&desk.pool)
    .await
    .unwrap();
    assert_eq!(notices, 1);
    let stranger_notices: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM notifications WHERE user_id = $1",
    )
    .bind(stranger)
    .fetch_one(&desk.pool)
    .await
    .unwrap();
    assert_eq!(stranger_notices, 0);

    let (status, listed) = call(
        &desk.app,
        authed("GET", "/api/v1/notifications", &owner_cookie, None),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{listed}");
    assert_eq!(listed["data"][0]["title"], "Amaka");

    sqlx::query("DELETE FROM messages WHERE mailbox_id = ANY($1)")
        .bind(vec![mailbox, hidden])
        .execute(&desk.pool)
        .await
        .ok();
    sqlx::query("DELETE FROM mailboxes WHERE id = ANY($1)")
        .bind(vec![mailbox, hidden])
        .execute(&desk.pool)
        .await
        .ok();
    sqlx::query("DELETE FROM users WHERE id = ANY($1)")
        .bind(vec![owner, stranger])
        .execute(&desk.pool)
        .await
        .ok();
}

fn mail(address: &str, provider_id: &str, subject: &str) -> InboundEmail {
    InboundEmail {
        provider_message_id: provider_id.to_string(),
        from_address: "amaka@northwind.example".to_string(),
        from_name: Some("Amaka".to_string()),
        to: vec![address.to_string()],
        cc: Vec::new(),
        subject: subject.to_string(),
        text: Some("The harbour invoice is ready.".to_string()),
        html: None,
        message_id_header: Some(format!("<{provider_id}@example.test>")),
        in_reply_to: None,
        references_header: None,
        received_at: Some(Utc::now()),
        attachment_count: 0,
    }
}

async fn create_user(pool: &PgPool, email: &str) -> Uuid {
    let id = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO users (email, password_hash, display_name) VALUES ($1, $2, 'Reader') RETURNING id",
    )
    .bind(email)
    .bind(hash_password("long-enough-password").unwrap())
    .fetch_one(pool)
    .await
    .unwrap();
    sqlx::query("INSERT INTO user_roles (user_id, role_id) VALUES ($1, $2)")
        .bind(id)
        .bind(Uuid::parse_str("10000000-0000-4000-8000-000000000001").unwrap())
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
