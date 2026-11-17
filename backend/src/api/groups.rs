use axum::Json;
use axum::extract::{Multipart, Path, State};
use axum::http::StatusCode;
use serde::Deserialize;
use uuid::Uuid;

use crate::api::body::Body;
use crate::api::errors::AppError;
use crate::api::response::ApiData;
use crate::auth::CurrentUser;
use crate::groups::{self, GroupImport, GroupInput, GroupView};
use crate::state::AppState;

#[derive(Deserialize)]
pub struct GroupBody {
    name: String,
    #[serde(default)]
    mailbox_ids: Vec<Uuid>,
}

#[derive(Deserialize)]
pub struct GroupPatch {
    name: Option<String>,
    mailbox_ids: Option<Vec<Uuid>>,
}

#[derive(Deserialize)]
pub struct MemberBody {
    mailbox_id: Uuid,
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

pub async fn add_group_mailbox(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<Uuid>,
    Body(body): Body<MemberBody>,
) -> Result<(StatusCode, Json<ApiData<GroupView>>), AppError> {
    let group = groups::add_mailbox(&state, user.id, id, body.mailbox_id).await?;
    Ok((StatusCode::CREATED, Json(ApiData { data: group })))
}

pub async fn remove_group_mailbox(
    State(state): State<AppState>,
    user: CurrentUser,
    Path((id, mailbox_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<ApiData<GroupView>>, AppError> {
    let group = groups::remove_mailbox(&state, user.id, id, mailbox_id).await?;
    Ok(Json(ApiData { data: group }))
}

pub async fn import_group(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<Uuid>,
    mut multipart: Multipart,
) -> Result<Json<ApiData<GroupImport>>, AppError> {
    let mut file = None;
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|_| AppError::Validation("The upload could not be read.".into()))?
    {
        if field.name() != Some("file") {
            continue;
        }
        let filename = field.file_name().unwrap_or("members.csv").to_string();
        let bytes = field
            .bytes()
            .await
            .map_err(|_| AppError::Validation("The upload could not be read.".into()))?;
        file = Some((filename, bytes));
        break;
    }
    let (filename, bytes) =
        file.ok_or_else(|| AppError::Validation("Choose a CSV or Excel file.".into()))?;
    Ok(Json(ApiData {
        data: groups::import_mailboxes(&state, user.id, id, &filename, &bytes).await?,
    }))
}

pub async fn delete_group(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    groups::delete(&state, user.id, id).await?;
    Ok(StatusCode::NO_CONTENT)
}
