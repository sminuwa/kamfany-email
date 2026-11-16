use axum::extract::{Request, State};
use axum::middleware::Next;
use axum::response::Response;

use crate::api::errors::AppError;
use crate::auth::browser_request_allowed;
use crate::state::AppState;

pub async fn require_browser(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Result<Response, AppError> {
    if request.method() != axum::http::Method::GET && request.method() != axum::http::Method::HEAD {
        let origin = request
            .headers()
            .get(axum::http::header::ORIGIN)
            .and_then(|value| value.to_str().ok());
        let marker = request
            .headers()
            .get("x-kamfany-request")
            .and_then(|value| value.to_str().ok());
        if !browser_request_allowed(origin, marker, &state.config.cors_origins) {
            return Err(AppError::Forbidden("This request was blocked.".into()));
        }
    }
    Ok(next.run(request).await)
}
