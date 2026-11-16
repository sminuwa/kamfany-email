use axum::Json;
use axum::extract::{Multipart, Path, Query, State};
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::IntoResponse;
use serde::Deserialize;
use uuid::Uuid;

use crate::api::body::Body;
use crate::api::errors::AppError;
use crate::api::response::ApiData;
use crate::auth::CurrentUser;
use crate::messages::compose::{
    DraftInput, create_draft, create_signature, delete_draft, delete_signature, download,
    list_signatures, read_draft, remove_file, suggest, update_draft, update_signature, upload_file,
};
use crate::state::AppState;

#[derive(Deserialize)]
pub struct DraftBody {
    #[serde(default = "new_kind")]
    kind: String,
    in_reply_to: Option<Uuid>,
    #[serde(default)]
    to: Vec<String>,
    #[serde(default)]
    cc: Vec<String>,
    #[serde(default)]
    bcc: Vec<String>,
    #[serde(default)]
    subject: String,
    #[serde(default)]
    html: String,
}

#[derive(Deserialize)]
pub struct SignatureBody {
    name: String,
    #[serde(default)]
    html: String,
    #[serde(default)]
    mailbox_id: Option<Uuid>,
    #[serde(default)]
    is_default: bool,
}

#[derive(Deserialize)]
pub struct SuggestQuery {
    #[serde(default)]
    q: String,
}

fn new_kind() -> String {
    "new".to_string()
}

fn input_of(body: DraftBody) -> DraftInput {
    DraftInput {
        kind: body.kind,
        in_reply_to: body.in_reply_to,
        to: body.to,
        cc: body.cc,
        bcc: body.bcc,
        subject: body.subject,
        html: body.html,
    }
}

pub async fn add_draft(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<Uuid>,
    Body(body): Body<DraftBody>,
) -> Result<impl IntoResponse, AppError> {
    let draft = create_draft(&state, &user, id, input_of(body)).await?;
    Ok((StatusCode::CREATED, Json(ApiData { data: draft })))
}

pub async fn open_draft(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiData<crate::messages::compose::DraftView>>, AppError> {
    Ok(Json(ApiData {
        data: read_draft(&state, &user, id).await?,
    }))
}

pub async fn save_draft(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<Uuid>,
    Body(body): Body<DraftBody>,
) -> Result<Json<ApiData<crate::messages::compose::DraftView>>, AppError> {
    Ok(Json(ApiData {
        data: update_draft(&state, &user, id, input_of(body)).await?,
    }))
}

pub async fn discard_draft(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    delete_draft(&state, &user, id).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn add_file(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<Uuid>,
    multipart: Multipart,
) -> Result<impl IntoResponse, AppError> {
    let file = upload_file(&state, &user, id, multipart).await?;
    Ok((StatusCode::CREATED, Json(ApiData { data: file })))
}

pub async fn discard_file(
    State(state): State<AppState>,
    user: CurrentUser,
    Path((id, file_id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode, AppError> {
    remove_file(&state, &user, id, file_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn open_file(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let file = download(&state, &user, id).await?;
    let disposition = format!(
        "{}; filename=\"{}\"",
        if file.inline { "inline" } else { "attachment" },
        file.filename.replace(['"', '\\', '\r', '\n'], "")
    );
    let mut headers = axum::http::HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_str(&file.content_type)
            .unwrap_or(HeaderValue::from_static("application/octet-stream")),
    );
    headers.insert(
        header::CONTENT_DISPOSITION,
        HeaderValue::from_str(&disposition)
            .unwrap_or(HeaderValue::from_static("attachment")),
    );
    headers.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("private, no-store"),
    );
    Ok((headers, file.bytes))
}

pub async fn mailbox_signatures(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiData<Vec<crate::messages::compose::SignatureView>>>, AppError> {
    Ok(Json(ApiData {
        data: list_signatures(&state, &user, id).await?,
    }))
}

pub async fn add_signature(
    State(state): State<AppState>,
    user: CurrentUser,
    Body(body): Body<SignatureBody>,
) -> Result<impl IntoResponse, AppError> {
    let signature = create_signature(
        &state,
        &user,
        body.mailbox_id,
        body.name,
        body.html,
        body.is_default,
    )
    .await?;
    Ok((StatusCode::CREATED, Json(ApiData { data: signature })))
}

pub async fn save_signature(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<Uuid>,
    Body(body): Body<SignatureBody>,
) -> Result<Json<ApiData<crate::messages::compose::SignatureView>>, AppError> {
    Ok(Json(ApiData {
        data: update_signature(&state, &user, id, body.name, body.html, body.is_default).await?,
    }))
}

pub async fn discard_signature(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    delete_signature(&state, &user, id).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn recipient_suggestions(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<Uuid>,
    Query(query): Query<SuggestQuery>,
) -> Result<Json<ApiData<Vec<crate::messages::compose::Suggestion>>>, AppError> {
    Ok(Json(ApiData {
        data: suggest(&state, &user, id, &query.q).await?,
    }))
}
