use axum::extract::Request;
use axum::http::{HeaderName, HeaderValue, Method, header};
use axum::middleware::Next;
use axum::response::Response;
use tower_http::cors::{AllowOrigin, CorsLayer};

use crate::config::ConfigError;

pub fn cors_layer(origins: &[String]) -> Result<CorsLayer, ConfigError> {
    let mut values = Vec::with_capacity(origins.len());
    for origin in origins {
        let value =
            HeaderValue::from_str(origin).map_err(|_| ConfigError::Invalid("CORS_ORIGINS"))?;
        values.push(value);
    }

    Ok(CorsLayer::new()
        .allow_origin(AllowOrigin::list(values))
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PATCH,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers([
            header::CONTENT_TYPE,
            header::AUTHORIZATION,
            header::ACCEPT,
            HeaderName::from_static("x-kamfany-request"),
        ])
        .allow_credentials(true))
}

pub async fn security_headers(request: Request, next: Next) -> Response {
    let mut response = next.run(request).await;
    let headers = response.headers_mut();
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    headers.insert(header::X_FRAME_OPTIONS, HeaderValue::from_static("DENY"));
    headers.insert(
        header::REFERRER_POLICY,
        HeaderValue::from_static("no-referrer"),
    );
    response
}
