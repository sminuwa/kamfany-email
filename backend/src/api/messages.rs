use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde::Deserialize;
use uuid::Uuid;

use crate::api::body::Body;
use crate::api::errors::AppError;
use crate::api::response::ApiData;
use crate::auth::{CurrentUser, pool_of};
use crate::messages::actions::{BulkChange, Destination};
use crate::messages::listing::ListQuery;
use crate::messages::reply::ComposeFields;
use crate::messages::{
    add_label, bulk, create_folder, create_label, delete_folder, delete_label, forward_message,
    get_message, list_folders, list_labels, list_messages, mark_read, mark_star, move_messages,
    remove_label, reply_message, update_folder, update_label,
};
use crate::state::AppState;

#[derive(Deserialize)]
pub struct MessageQuery {
    view: Option<String>,
    folder_id: Option<Uuid>,
    cursor: Option<String>,
}

#[derive(Deserialize)]
pub struct NameBody {
    name: String,
    color: Option<String>,
}

#[derive(Deserialize)]
pub struct NamePatch {
    name: Option<String>,
    color: Option<String>,
}

#[derive(Deserialize)]
pub struct ReadBody {
    read: bool,
}

#[derive(Deserialize)]
pub struct StarBody {
    starred: bool,
}

#[derive(Deserialize)]
pub struct MoveBody {
    folder_id: Option<Uuid>,
    system_key: Option<String>,
}

#[derive(Deserialize)]
pub struct LabelBody {
    label_id: Uuid,
}

#[derive(Deserialize)]
pub struct BulkBody {
    ids: Vec<Uuid>,
    action: String,
    read: Option<bool>,
    starred: Option<bool>,
    folder_id: Option<Uuid>,
    system_key: Option<String>,
    label_id: Option<Uuid>,
}

#[derive(Deserialize)]
pub struct ReplyBody {
    #[serde(default)]
    all: bool,
    to: Option<Vec<String>>,
    cc: Option<Vec<String>>,
    bcc: Option<Vec<String>>,
    subject: Option<String>,
    text: Option<String>,
    html: Option<String>,
    draft_id: Option<Uuid>,
}

#[derive(Deserialize)]
pub struct ForwardBody {
    to: Option<Vec<String>>,
    cc: Option<Vec<String>>,
    bcc: Option<Vec<String>>,
    subject: Option<String>,
    text: Option<String>,
    html: Option<String>,
    draft_id: Option<Uuid>,
}

#[derive(serde::Serialize)]
pub struct SentView {
    id: Uuid,
    status: String,
    provider_message_id: String,
}

pub async fn list_mailbox_messages(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<Uuid>,
    Query(query): Query<MessageQuery>,
) -> Result<Json<ApiData<crate::messages::listing::MessagePage>>, AppError> {
    let page = list_messages(
        pool_of(&state)?,
        id,
        &user,
        ListQuery {
            view: query.view,
            folder_id: query.folder_id,
            cursor: query.cursor,
        },
    )
    .await?;
    Ok(Json(ApiData { data: page }))
}

pub async fn bulk_messages(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<Uuid>,
    Body(body): Body<BulkBody>,
) -> Result<StatusCode, AppError> {
    bulk(
        pool_of(&state)?,
        &user,
        id,
        BulkChange {
            ids: body.ids,
            action: body.action,
            read: body.read,
            starred: body.starred,
            folder_id: body.folder_id,
            system_key: body.system_key,
            label_id: body.label_id,
        },
        &state.storage,
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn read_message(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiData<crate::messages::listing::MessageView>>, AppError> {
    let view = get_message(pool_of(&state)?, id, &user).await?;
    Ok(Json(ApiData { data: view }))
}

pub async fn set_read(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<Uuid>,
    Body(body): Body<ReadBody>,
) -> Result<StatusCode, AppError> {
    let pool = pool_of(&state)?;
    let mailbox_id = crate::messages::access::message_mailbox(pool, id).await?;
    mark_read(pool, &user, mailbox_id, vec![id], body.read)
        .await
        .map_err(hide)?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn set_star(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<Uuid>,
    Body(body): Body<StarBody>,
) -> Result<StatusCode, AppError> {
    let pool = pool_of(&state)?;
    let mailbox_id = crate::messages::access::message_mailbox(pool, id).await?;
    mark_star(pool, &user, mailbox_id, vec![id], body.starred)
        .await
        .map_err(hide)?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn archive_message(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    move_one(
        &state,
        &user,
        id,
        Destination::System("archive".to_string()),
    )
    .await
}

pub async fn move_message(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<Uuid>,
    Body(body): Body<MoveBody>,
) -> Result<StatusCode, AppError> {
    let destination = match (body.folder_id, body.system_key) {
        (Some(folder_id), None) => Destination::Folder(folder_id),
        (None, Some(key)) => Destination::System(key),
        _ => return Err(AppError::Validation("Choose a folder to move to.".into())),
    };
    move_one(&state, &user, id, destination).await
}

pub async fn remove_message(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    let pool = pool_of(&state)?;
    let mailbox_id = crate::messages::access::message_mailbox(pool, id).await?;
    crate::messages::actions::discard_messages(pool, &user, mailbox_id, vec![id], &state.storage)
        .await
        .map_err(hide)?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn label_message(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<Uuid>,
    Body(body): Body<LabelBody>,
) -> Result<StatusCode, AppError> {
    let pool = pool_of(&state)?;
    let mailbox_id = crate::messages::access::message_mailbox(pool, id).await?;
    add_label(pool, &user, mailbox_id, vec![id], body.label_id)
        .await
        .map_err(hide)?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn unlabel_message(
    State(state): State<AppState>,
    user: CurrentUser,
    Path((id, label_id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode, AppError> {
    let pool = pool_of(&state)?;
    let mailbox_id = crate::messages::access::message_mailbox(pool, id).await?;
    remove_label(pool, &user, mailbox_id, vec![id], label_id)
        .await
        .map_err(hide)?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn reply(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<Uuid>,
    Body(body): Body<ReplyBody>,
) -> Result<impl IntoResponse, AppError> {
    let sent = reply_message(
        &state,
        &user,
        id,
        body.all,
        ComposeFields {
            to: body.to,
            cc: body.cc,
            bcc: body.bcc,
            subject: body.subject,
            text: body.text,
            html: body.html,
            draft_id: body.draft_id,
        },
    )
    .await?;
    Ok((
        StatusCode::CREATED,
        Json(ApiData {
            data: sent_view(sent),
        }),
    ))
}

pub async fn forward(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<Uuid>,
    Body(body): Body<ForwardBody>,
) -> Result<impl IntoResponse, AppError> {
    let sent = forward_message(
        &state,
        &user,
        id,
        ComposeFields {
            to: body.to,
            cc: body.cc,
            bcc: body.bcc,
            subject: body.subject,
            text: body.text,
            html: body.html,
            draft_id: body.draft_id,
        },
    )
    .await?;
    Ok((
        StatusCode::CREATED,
        Json(ApiData {
            data: sent_view(sent),
        }),
    ))
}

pub async fn folders(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiData<Vec<crate::messages::catalog::FolderItem>>>, AppError> {
    let items = list_folders(pool_of(&state)?, id, &user).await?;
    Ok(Json(ApiData { data: items }))
}

pub async fn add_folder(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<Uuid>,
    Body(body): Body<NameBody>,
) -> Result<impl IntoResponse, AppError> {
    let item = create_folder(
        pool_of(&state)?,
        id,
        &user,
        &body.name,
        body.color.as_deref(),
    )
    .await?;
    Ok((StatusCode::CREATED, Json(ApiData { data: item })))
}

pub async fn patch_folder(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<Uuid>,
    Body(body): Body<NamePatch>,
) -> Result<Json<ApiData<crate::messages::catalog::FolderItem>>, AppError> {
    let item = update_folder(
        pool_of(&state)?,
        id,
        &user,
        body.name.as_deref(),
        body.color.as_deref(),
    )
    .await?;
    Ok(Json(ApiData { data: item }))
}

pub async fn remove_folder(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    delete_folder(pool_of(&state)?, id, &user).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn labels(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiData<Vec<crate::messages::catalog::LabelItem>>>, AppError> {
    let items = list_labels(pool_of(&state)?, id, &user).await?;
    Ok(Json(ApiData { data: items }))
}

pub async fn add_label_def(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<Uuid>,
    Body(body): Body<NameBody>,
) -> Result<impl IntoResponse, AppError> {
    let item = create_label(
        pool_of(&state)?,
        id,
        &user,
        &body.name,
        body.color.as_deref(),
    )
    .await?;
    Ok((StatusCode::CREATED, Json(ApiData { data: item })))
}

pub async fn patch_label(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<Uuid>,
    Body(body): Body<NamePatch>,
) -> Result<Json<ApiData<crate::messages::catalog::LabelItem>>, AppError> {
    let item = update_label(
        pool_of(&state)?,
        id,
        &user,
        body.name.as_deref(),
        body.color.as_deref(),
    )
    .await?;
    Ok(Json(ApiData { data: item }))
}

pub async fn remove_label_def(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    delete_label(pool_of(&state)?, id, &user).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn move_one(
    state: &AppState,
    user: &CurrentUser,
    message_id: Uuid,
    destination: Destination,
) -> Result<StatusCode, AppError> {
    let pool = pool_of(state)?;
    let mailbox_id = crate::messages::access::message_mailbox(pool, message_id).await?;
    move_messages(pool, user, mailbox_id, vec![message_id], destination)
        .await
        .map_err(hide)?;
    Ok(StatusCode::NO_CONTENT)
}

fn sent_view(sent: crate::delivery::SentMessage) -> SentView {
    SentView {
        id: sent.id,
        status: sent.status,
        provider_message_id: sent.provider_message_id,
    }
}

fn hide(error: AppError) -> AppError {
    match error {
        AppError::Forbidden(_) => AppError::NotFound,
        other => other,
    }
}
