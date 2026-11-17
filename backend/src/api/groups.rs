use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use serde::Deserialize;
use uuid::Uuid;

use crate::api::body::Body;
use crate::api::errors::AppError;
use crate::api::response::ApiData;
use crate::auth::CurrentUser;
use crate::groups::{self, GroupInput, GroupView};
use crate::state::AppState;

#[derive(Deserialize)]
pub struct GroupBody {
    name: String,
    address: String,
    mailbox_ids: Vec<Uuid>,
}

#[derive(Deserialize)]
pub struct GroupPatch {
    name: Option<String>,
    mailbox_ids: Option<Vec<Uuid>>,
}

pub async fn list_groups(
    State(state): State<AppState>,
    _user: CurrentUser,
) -> Result<Json<ApiData<Vec<GroupView>>>, AppError> {
    Ok(Json(ApiData {
        data: groups::list(&state).await?,
    }))
}

pub async fn create_group(
    State(state): State<AppState>,
    user: CurrentUser,
    Body(body): Body<GroupBody>,
) -> Result<(StatusCode, Json<ApiData<GroupView>>), AppError> {
    let group = groups::create(
        &state,
        user.id,
        GroupInput {
            name: body.name,
            local_part: body.address,
            mailbox_ids: body.mailbox_ids,
        },
    )
    .await?;
    Ok((StatusCode::CREATED, Json(ApiData { data: group })))
}

pub async fn update_group(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<Uuid>,
    Body(body): Body<GroupPatch>,
) -> Result<Json<ApiData<GroupView>>, AppError> {
    let group = groups::update(&state, user.id, id, body.name, body.mailbox_ids).await?;
    Ok(Json(ApiData { data: group }))
}

pub async fn delete_group(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    groups::delete(&state, user.id, id).await?;
    Ok(StatusCode::NO_CONTENT)
}
