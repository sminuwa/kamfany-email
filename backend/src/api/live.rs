use axum::Json;
use axum::extract::ws::WebSocketUpgrade;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::IntoResponse;
use uuid::Uuid;

use crate::api::errors::AppError;
use crate::api::response::ApiData;
use crate::auth::{CurrentUser, SESSION_COOKIE, hash_token, pool_of, read_cookie};
use crate::live::{self, Notice};
use crate::state::AppState;

pub async fn notices(
    State(state): State<AppState>,
    user: CurrentUser,
) -> Result<Json<ApiData<Vec<Notice>>>, AppError> {
    Ok(Json(ApiData {
        data: live::list_notices(&state, user.id).await?,
    }))
}

pub async fn read_notice(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    live::mark_notice(&state, user.id, id).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn read_notices(
    State(state): State<AppState>,
    user: CurrentUser,
) -> Result<StatusCode, AppError> {
    live::mark_all(&state, user.id).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn socket(
    State(state): State<AppState>,
    headers: HeaderMap,
    upgrade: WebSocketUpgrade,
) -> Result<impl IntoResponse, AppError> {
    let origin = headers
        .get(header::ORIGIN)
        .and_then(|value| value.to_str().ok());
    let allowed =
        origin.is_some_and(|value| state.config.cors_origins.iter().any(|item| item == value));
    if !allowed {
        return Err(AppError::Forbidden(
            "This connection is not from the desk.".into(),
        ));
    }
    let token = read_cookie(&headers, SESSION_COOKIE).ok_or(AppError::Unauthorized)?;
    let user = crate::auth::user_from_token(pool_of(&state)?, &hash_token(&token))
        .await?
        .ok_or(AppError::Unauthorized)?;
    let notices = state.live.subscribe(user.id);
    Ok(upgrade.on_upgrade(move |socket| live::drive(socket, notices)))
}
