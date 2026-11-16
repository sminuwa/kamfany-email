use axum::Json;
use axum::body::Bytes;
use axum::extract::{Multipart, Path, Query, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::IntoResponse;
use serde::Deserialize;
use uuid::Uuid;

use crate::api::body::Body;
use crate::api::errors::AppError;
use crate::api::response::ApiData;
use crate::auth::CurrentUser;
use crate::contacts::{
    ContactInput, create_contact, delete_contact, import_contacts, list_contacts, open_avatar,
    remove_avatar, save_avatar, update_contact,
};
use crate::state::AppState;

#[derive(Deserialize)]
pub struct ContactQuery {
    q: Option<String>,
}

#[derive(Deserialize)]
pub struct ContactBody {
    full_name: String,
    email: Option<String>,
    phone: Option<String>,
    company: Option<String>,
    notes: Option<String>,
}

pub async fn contacts(
    State(state): State<AppState>,
    user: CurrentUser,
    Query(query): Query<ContactQuery>,
) -> Result<Json<ApiData<Vec<crate::contacts::Contact>>>, AppError> {
    Ok(Json(ApiData {
        data: list_contacts(&state, &user, query.q.as_deref().unwrap_or("")).await?,
    }))
}

pub async fn add_contact(
    State(state): State<AppState>,
    user: CurrentUser,
    Body(body): Body<ContactBody>,
) -> Result<impl IntoResponse, AppError> {
    let contact = create_contact(&state, &user, input_of(body)).await?;
    Ok((StatusCode::CREATED, Json(ApiData { data: contact })))
}

pub async fn save_contact(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<Uuid>,
    Body(body): Body<ContactBody>,
) -> Result<Json<ApiData<crate::contacts::Contact>>, AppError> {
    Ok(Json(ApiData {
        data: update_contact(&state, &user, id, input_of(body)).await?,
    }))
}

pub async fn discard_contact(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    delete_contact(&state, &user, id).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn import(
    State(state): State<AppState>,
    user: CurrentUser,
    mut multipart: Multipart,
) -> Result<Json<ApiData<crate::contacts::ImportResult>>, AppError> {
    let mut bytes = None;
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|_| AppError::Validation("The upload could not be read.".into()))?
    {
        if field.name() != Some("file") {
            continue;
        }
        bytes = Some(
            field
                .bytes()
                .await
                .map_err(|_| AppError::Validation("The upload could not be read.".into()))?,
        );
        break;
    }
    let bytes = bytes.ok_or_else(|| AppError::Validation("Choose a CSV file.".into()))?;
    Ok(Json(ApiData {
        data: import_contacts(&state, &user, &bytes).await?,
    }))
}

pub async fn add_avatar(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<Uuid>,
    mut multipart: Multipart,
) -> Result<Json<ApiData<crate::contacts::Contact>>, AppError> {
    let mut file = None;
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|_| AppError::Validation("The upload could not be read.".into()))?
    {
        if field.name() != Some("file") {
            continue;
        }
        let mime = field
            .content_type()
            .unwrap_or("application/octet-stream")
            .to_string();
        let bytes = field
            .bytes()
            .await
            .map_err(|_| AppError::Validation("The upload could not be read.".into()))?;
        file = Some((mime, bytes));
        break;
    }
    let (mime, bytes) = file.ok_or_else(|| AppError::Validation("Choose an image.".into()))?;
    Ok(Json(ApiData {
        data: save_avatar(&state, &user, id, &mime, &bytes).await?,
    }))
}

pub async fn discard_avatar(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    remove_avatar(&state, &user, id).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn avatar(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let file = open_avatar(&state, &user, id).await?;
    let mut headers = HeaderMap::new();
    headers.insert(header::CONTENT_TYPE, file.content_type.parse().unwrap_or(header::HeaderValue::from_static("application/octet-stream")));
    headers.insert(
        header::CACHE_CONTROL,
        header::HeaderValue::from_static("private, no-store"),
    );
    Ok((headers, Bytes::from(file.bytes)))
}

fn input_of(body: ContactBody) -> ContactInput {
    ContactInput {
        full_name: body.full_name,
        email: body.email,
        phone: body.phone,
        company: body.company,
        notes: body.notes,
    }
}
