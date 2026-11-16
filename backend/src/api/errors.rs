use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("The page or record was not found.")]
    NotFound,
    #[error("{0}")]
    Validation(String),
    #[error("Sign in is required.")]
    Unauthorized,
    #[error("Email or password is incorrect.")]
    InvalidCredentials,
    #[error("{0}")]
    Forbidden(String),
    #[error("{0}")]
    Conflict(String),
    #[error("{0}")]
    Provider(String),
    #[error("{0}")]
    Limited(String),
    #[error("{0}")]
    Unavailable(String),
    #[error("Database is unavailable.")]
    NotReady,
    #[error(transparent)]
    Database(#[from] sqlx::Error),
    #[error("Something went wrong. Try again.")]
    Internal,
}

#[derive(Serialize)]
struct ErrorBody {
    error: ErrorDetail,
}

#[derive(Serialize)]
struct ErrorDetail {
    code: &'static str,
    message: String,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, code, message) = match &self {
            AppError::NotFound => (StatusCode::NOT_FOUND, "not_found", self.to_string()),
            AppError::Validation(message) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "validation",
                message.clone(),
            ),
            AppError::Unauthorized => (StatusCode::UNAUTHORIZED, "unauthorized", self.to_string()),
            AppError::InvalidCredentials => {
                (StatusCode::UNAUTHORIZED, "unauthorized", self.to_string())
            }
            AppError::Forbidden(message) => (StatusCode::FORBIDDEN, "forbidden", message.clone()),
            AppError::Conflict(message) => (StatusCode::CONFLICT, "conflict", message.clone()),
            AppError::Provider(message) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "provider",
                message.clone(),
            ),
            AppError::Limited(message) => (
                StatusCode::TOO_MANY_REQUESTS,
                "rate_limited",
                message.clone(),
            ),
            AppError::Unavailable(message) => (
                StatusCode::SERVICE_UNAVAILABLE,
                "unavailable",
                message.clone(),
            ),
            AppError::NotReady => (
                StatusCode::SERVICE_UNAVAILABLE,
                "not_ready",
                self.to_string(),
            ),
            AppError::Database(error) => {
                tracing::error!(%error, "database request failed");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "internal",
                    "Something went wrong. Try again.".to_string(),
                )
            }
            AppError::Internal => {
                tracing::error!("internal error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "internal",
                    self.to_string(),
                )
            }
        };

        (
            status,
            Json(ErrorBody {
                error: ErrorDetail { code, message },
            }),
        )
            .into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::AppError;
    use axum::http::StatusCode;
    use axum::response::IntoResponse;
    use http_body_util::BodyExt;

    async fn error_json(error: AppError) -> (StatusCode, serde_json::Value) {
        let response = error.into_response();
        let status = response.status();
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        let json = serde_json::from_slice(&bytes).unwrap();
        (status, json)
    }

    #[tokio::test]
    async fn not_found_uses_the_error_envelope() {
        let (status, json) = error_json(AppError::NotFound).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(json["error"]["code"], "not_found");
    }

    #[tokio::test]
    async fn validation_keeps_the_caller_message() {
        let (status, json) = error_json(AppError::Validation("Subject is required.".into())).await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(json["error"]["code"], "validation");
        assert_eq!(json["error"]["message"], "Subject is required.");
    }

    #[tokio::test]
    async fn unauthorized_does_not_invent_a_session() {
        let (status, json) = error_json(AppError::Unauthorized).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert_eq!(json["error"]["code"], "unauthorized");
    }

    #[tokio::test]
    async fn database_failures_stay_generic() {
        let (status, json) = error_json(AppError::Database(sqlx::Error::PoolClosed)).await;
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(json["error"]["code"], "internal");
        assert_eq!(json["error"]["message"], "Something went wrong. Try again.");
        assert!(!json.to_string().contains("PoolClosed"));
    }

    #[tokio::test]
    async fn internal_errors_stay_generic() {
        let (status, json) = error_json(AppError::Internal).await;
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(json["error"]["message"], "Something went wrong. Try again.");
    }
}
