use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;

use crate::api::errors::AppError;
use crate::api::response::ApiData;
use crate::state::AppState;

#[derive(Serialize)]
struct HealthData {
    status: &'static str,
    service: &'static str,
    version: &'static str,
    database: &'static str,
}

pub async fn health(State(state): State<AppState>) -> impl IntoResponse {
    let connected = state.database_connected().await;
    Json(ApiData {
        data: snapshot("ok", connected),
    })
}

pub async fn ready(State(state): State<AppState>) -> Response {
    if state.database_connected().await {
        (
            StatusCode::OK,
            Json(ApiData {
                data: snapshot("ready", true),
            }),
        )
            .into_response()
    } else {
        AppError::NotReady.into_response()
    }
}

fn snapshot(status: &'static str, connected: bool) -> HealthData {
    HealthData {
        status,
        service: "kamfany-mail",
        version: env!("CARGO_PKG_VERSION"),
        database: if connected {
            "connected"
        } else {
            "unavailable"
        },
    }
}
