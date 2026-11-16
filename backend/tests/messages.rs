use std::time::Duration;

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use chrono::Utc;
use http_body_util::BodyExt;
use kamfany_mail::auth::hash_password;
use kamfany_mail::config::AppConfig;
use kamfany_mail::delivery::store_inbound;
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
    id
}

fn call(method: &str, uri: &str, cookie: &str, body: Option<serde_json::Value>) -> Request<Body> {
    let bytes = *Uuid::new_v4().as_bytes();
    let mut builder = Request::builder()
        .method(method)
        .uri(uri)
        .header(header::ORIGIN, ORIGIN)
        .header("x-kamfany-request", "1")
        .header(
            "x-forwarded-for",
            format!("10.{}.{}.{}", bytes[13], bytes[14], bytes[15]),
        )
        .header(header::COOKIE, format!("kamfany_session={cookie}"));
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

fn inbound(
    address: &str,
    provider_id: &str,
    header: &str,
    reply_to: Option<&str>,
    subject: &str,
) -> InboundEmail {
    InboundEmail {
        provider_message_id: provider_id.to_string(),
        from_address: "amaka@northwind.example".to_string(),
        from_name: Some("Amaka".to_string()),
        to: vec![address.to_string()],
        cc: vec!["info@kamfany.test".to_string()],
        subject: subject.to_string(),
        text: Some("The harbour invoice is ready.".to_string()),
        html: None,
        message_id_header: Some(header.to_string()),
        in_reply_to: reply_to.map(str::to_string),
        references_header: reply_to.map(str::to_string),
        received_at: Some(Utc::now()),
        attachment_count: 0,
    }
}

#[tokio::test]
async fn stored_mail_can_be_filed_threaded_and_labelled() {
    let desk = desk().await;
    let suffix = Uuid::new_v4().simple();
    let owner_email = format!("owner-{suffix}@kamfany.test");
    let stranger_email = format!("stranger-{suffix}@kamfany.test");
    let address = format!("desk-{suffix}@kamfany.test");
    let owner_role = uuid::uuid!("10000000-0000-4000-8000-000000000001");
    let member_role = uuid::uuid!("10000000-0000-4000-8000-000000000003");
    let owner_id = create_user(&desk.pool, &owner_email, owner_role).await;
    let stranger_id = create_user(&desk.pool, &stranger_email, member_role).await;

    let (status, _, owner_cookie) = send(
        &desk.app,
        call(
            "POST",
            "/api/v1/auth/login",
            "",
            Some(json!({ "email": owner_email, "password": "long-enough-password" })),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let owner_cookie = owner_cookie.expect("owner session");

    let (status, created, _) = send(
        &desk.app,
        call(
            "POST",
            "/api/v1/mailboxes",
            &owner_cookie,
            Some(json!({ "address": address, "display_name": "Desk" })),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let mailbox_id = created["data"]["id"].as_str().expect("mailbox").to_string();

    let first_header = format!("<first-{suffix}@example.test>");
    let second_header = format!("<second-{suffix}@example.test>");
    store_inbound(
        &desk.pool,
        &inbound(
            &address,
            &format!("in-{suffix}-1"),
            &first_header,
            None,
            "Harbour invoice",
        ),
        &[],
    )
    .await
    .expect("first");
    store_inbound(
        &desk.pool,
        &inbound(
            &address,
            &format!("in-{suffix}-2"),
            &second_header,
            Some(&first_header),
            "Re: Harbour invoice",
        ),
        &[],
    )
    .await
    .expect("reply");

    let (status, page, _) = send(
        &desk.app,
        call(
            "GET",
            &format!("/api/v1/mailboxes/{mailbox_id}/messages?view=inbox"),
            &owner_cookie,
            None,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        page["data"]["messages"].as_array().expect("messages").len(),
        2
    );
    let message_id = page["data"]["messages"][0]["id"]
        .as_str()
        .expect("id")
        .to_string();
    let thread_id = page["data"]["messages"][0]["thread_id"]
        .as_str()
        .expect("thread");
    assert_eq!(
        page["data"]["messages"][1]["thread_id"].as_str(),
        Some(thread_id)
    );

    let (status, detail, _) = send(
        &desk.app,
        call(
            "GET",
            &format!("/api/v1/messages/{message_id}"),
            &owner_cookie,
            None,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        detail["data"]["thread"].as_array().expect("thread").len(),
        2
    );
    assert!(
        !detail["data"]["message"]["cc"]
            .as_array()
            .expect("cc")
            .is_empty()
    );

    let (status, _, stranger_cookie) = send(
        &desk.app,
        call(
            "POST",
            "/api/v1/auth/login",
            "",
            Some(json!({ "email": stranger_email, "password": "long-enough-password" })),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (status, _, _) = send(
        &desk.app,
        call(
            "GET",
            &format!("/api/v1/messages/{message_id}"),
            &stranger_cookie.expect("stranger"),
            None,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    let (status, _, _) = send(
        &desk.app,
        call(
            "POST",
            &format!("/api/v1/mailboxes/{mailbox_id}/messages"),
            &owner_cookie,
            Some(json!({ "ids": [message_id], "action": "read", "read": true })),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    let (status, folder, _) = send(
        &desk.app,
        call(
            "POST",
            &format!("/api/v1/mailboxes/{mailbox_id}/folders"),
            &owner_cookie,
            Some(json!({ "name": format!("Harbour {suffix}"), "color": "brass" })),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let folder_id = folder["data"]["id"].as_str().expect("folder");

    let (status, _, _) = send(
        &desk.app,
        call(
            "POST",
            &format!("/api/v1/messages/{message_id}/move"),
            &owner_cookie,
            Some(json!({ "folder_id": folder_id })),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    let (status, label, _) = send(
        &desk.app,
        call(
            "POST",
            &format!("/api/v1/mailboxes/{mailbox_id}/labels"),
            &owner_cookie,
            Some(json!({ "name": format!("Clients {suffix}"), "color": "paper" })),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let label_id = label["data"]["id"].as_str().expect("label");
    let (status, _, _) = send(
        &desk.app,
        call(
            "POST",
            &format!("/api/v1/messages/{message_id}/labels"),
            &owner_cookie,
            Some(json!({ "label_id": label_id })),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    let (status, archived, _) = send(
        &desk.app,
        call(
            "GET",
            &format!("/api/v1/mailboxes/{mailbox_id}/messages?folder_id={folder_id}"),
            &owner_cookie,
            None,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let filed = &archived["data"]["messages"][0];
    assert_eq!(filed["is_read"], true);
    assert_eq!(filed["label_ids"][0], label_id);

    let (status, folders, _) = send(
        &desk.app,
        call(
            "GET",
            &format!("/api/v1/mailboxes/{mailbox_id}/folders"),
            &owner_cookie,
            None,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let inbox = folders["data"]
        .as_array()
        .expect("folders")
        .iter()
        .find(|folder| folder["system_key"] == "inbox")
        .expect("inbox");
    let (status, _, _) = send(
        &desk.app,
        call(
            "DELETE",
            &format!("/api/v1/folders/{}", inbox["id"].as_str().unwrap()),
            &owner_cookie,
            None,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);

    let (status, missing, _) = send(
        &desk.app,
        call(
            "POST",
            &format!("/api/v1/messages/{message_id}/reply"),
            &owner_cookie,
            Some(json!({ "text": "Received.", "subject": "Re: Harbour invoice" })),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(missing["error"]["message"], "Resend is not configured.");

    sqlx::query("DELETE FROM messages WHERE mailbox_id = $1")
        .bind(Uuid::parse_str(&mailbox_id).unwrap())
        .execute(&desk.pool)
        .await
        .expect("message cleanup");
    sqlx::query("DELETE FROM mailboxes WHERE id = $1")
        .bind(Uuid::parse_str(&mailbox_id).unwrap())
        .execute(&desk.pool)
        .await
        .expect("mailbox cleanup");
    sqlx::query("DELETE FROM users WHERE id = ANY($1)")
        .bind(vec![owner_id, stranger_id])
        .execute(&desk.pool)
        .await
        .expect("user cleanup");
}
