use std::time::Duration;

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use kamfany_mail::api::AppError;
use kamfany_mail::auth::hash_password;
use kamfany_mail::config::AppConfig;
use kamfany_mail::limit::charge;
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
                .join("kamfany-mail-security-test")
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

fn peer() -> String {
    let bytes = *Uuid::new_v4().as_bytes();
    format!("10.{}.{}.{}", bytes[13], bytes[14], bytes[15])
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
                .header("x-forwarded-for", peer())
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

async fn send(
    app: &axum::Router,
    request: Request<Body>,
) -> (StatusCode, serde_json::Value, Option<String>) {
    let response = app.clone().oneshot(request).await.expect("response");
    let status = response.status();
    let nosniff = response
        .headers()
        .get(header::X_CONTENT_TYPE_OPTIONS)
        .and_then(|value| value.to_str().ok())
        .map(str::to_string);
    let bytes = response.into_body().collect().await.expect("body").to_bytes();
    let json = serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null);
    (status, json, nosniff)
}

async fn create_user(pool: &PgPool, email: &str, role: &str) -> Uuid {
    let id = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO users (email, password_hash, display_name) VALUES ($1, $2, 'Reviewer') RETURNING id",
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

#[tokio::test]
async fn audit_and_provider_stay_with_the_right_role() {
    let desk = desk().await;
    let suffix = Uuid::new_v4().simple();
    let owner_email = format!("owner-{suffix}@kamfany.test");
    let member_email = format!("member-{suffix}@kamfany.test");
    let owner = create_user(
        &desk.pool,
        &owner_email,
        "10000000-0000-4000-8000-000000000001",
    )
    .await;
    let member = create_user(
        &desk.pool,
        &member_email,
        "10000000-0000-4000-8000-000000000003",
    )
    .await;
    let owner_cookie = login(&desk, &owner_email).await;
    let member_cookie = login(&desk, &member_email).await;

    let (status, json, _) = send(
        &desk.app,
        Request::builder()
            .method("GET")
            .uri("/api/v1/audit")
            .header(header::ORIGIN, ORIGIN)
            .header("x-kamfany-request", "1")
            .header(header::COOKIE, format!("kamfany_session={member_cookie}"))
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{json}");

    let (status, json, _) = send(
        &desk.app,
        Request::builder()
            .method("GET")
            .uri("/api/v1/provider")
            .header(header::ORIGIN, ORIGIN)
            .header("x-kamfany-request", "1")
            .header(header::COOKIE, format!("kamfany_session={member_cookie}"))
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{json}");

    let (status, json, nosniff) = send(
        &desk.app,
        Request::builder()
            .method("GET")
            .uri("/api/v1/audit")
            .header(header::ORIGIN, ORIGIN)
            .header("x-kamfany-request", "1")
            .header(header::COOKIE, format!("kamfany_session={owner_cookie}"))
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{json}");
    assert_eq!(nosniff.as_deref(), Some("nosniff"));
    let actions: Vec<&str> = json["data"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|row| row["action"].as_str())
        .collect();
    assert!(actions.contains(&"auth.login"));

    let (status, json, _) = send(
        &desk.app,
        Request::builder()
            .method("GET")
            .uri("/api/v1/provider")
            .header(header::ORIGIN, ORIGIN)
            .header("x-kamfany-request", "1")
            .header(header::COOKIE, format!("kamfany_session={owner_cookie}"))
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{json}");
    assert_eq!(json["data"]["scanner"], "not_configured");
    assert!(json["data"]["queue_received"].is_number());

    let bucket = format!("test:rate:{}", Uuid::new_v4());
    charge(&desk.pool, &bucket, 60, 2).await.unwrap();
    charge(&desk.pool, &bucket, 60, 2).await.unwrap();
    let error = charge(&desk.pool, &bucket, 60, 2).await.unwrap_err();
    assert!(matches!(error, AppError::Limited(_)));

    sqlx::query("DELETE FROM users WHERE id = ANY($1)")
        .bind(vec![owner, member])
        .execute(&desk.pool)
        .await
        .unwrap();
}
