use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use kamfany_mail::config::AppConfig;
use kamfany_mail::router;
use kamfany_mail::state::AppState;
use tower::ServiceExt;

fn test_app() -> axum::Router {
    let config = AppConfig::from_lookup(|_| None).expect("defaults");
    router(AppState::without_database(config).expect("storage")).expect("router")
}

async fn json_body(response: axum::response::Response) -> serde_json::Value {
    let bytes = response
        .into_body()
        .collect()
        .await
        .expect("body")
        .to_bytes();
    serde_json::from_slice(&bytes).expect("json")
}

#[tokio::test]
async fn health_reports_liveness_without_a_database() {
    let response = test_app()
        .oneshot(
            Request::builder()
                .uri("/api/v1/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = json_body(response).await;
    assert_eq!(body["data"]["status"], "ok");
    assert_eq!(body["data"]["service"], "kamfany-mail");
    assert_eq!(body["data"]["version"], "0.1.0");
    assert_eq!(body["data"]["database"], "unavailable");
}

#[tokio::test]
async fn brand_reports_the_install_identity() {
    let response = test_app()
        .oneshot(
            Request::builder()
                .uri("/api/v1/brand")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = json_body(response).await;
    assert_eq!(body["data"]["name"], "Mail");
    assert_eq!(body["data"]["company"], "");
    assert_eq!(body["data"]["domain"], "kamfany.com");
    assert_eq!(body["data"]["public_url"], "");
    assert_eq!(body["data"]["logo_url"], "");
}

#[tokio::test]
async fn ready_is_unavailable_without_a_database() {
    let response = test_app()
        .oneshot(
            Request::builder()
                .uri("/api/v1/ready")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    let body = json_body(response).await;
    assert_eq!(body["error"]["code"], "not_ready");
    assert_eq!(body["error"]["message"], "Database is unavailable.");
}

#[tokio::test]
async fn unknown_routes_return_json() {
    let response = test_app()
        .oneshot(
            Request::builder()
                .uri("/api/v1/missing")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let body = json_body(response).await;
    assert_eq!(body["error"]["code"], "not_found");
}
