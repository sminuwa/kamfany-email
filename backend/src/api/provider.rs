use axum::Json;
use axum::body::Bytes;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::api::body::Body;
use crate::api::errors::AppError;
use crate::api::response::ApiData;
use crate::auth::CurrentUser;
use crate::delivery::{SendRequest, accept_webhook, provider_report, send_mail, sync_provider};
use crate::state::AppState;

#[derive(Deserialize)]
pub struct SendBody {
    to: Vec<String>,
    #[serde(default)]
    cc: Vec<String>,
    #[serde(default)]
    bcc: Vec<String>,
    subject: String,
    text: Option<String>,
    html: Option<String>,
    #[serde(default)]
    draft_id: Option<Uuid>,
    #[serde(default)]
    group_ids: Vec<Uuid>,
}

#[derive(Serialize)]
pub struct WebhookView {
    status: &'static str,
}

#[derive(Serialize)]
pub struct SendView {
    id: Uuid,
    status: String,
    provider_message_id: String,
}

pub async fn resend_webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response, AppError> {
    accept_webhook(&state, &headers, &body).await?;
    Ok((
        StatusCode::OK,
        Json(ApiData {
            data: WebhookView { status: "accepted" },
        }),
    )
        .into_response())
}

pub async fn provider_status(
    State(state): State<AppState>,
    user: CurrentUser,
) -> Result<Json<ApiData<crate::delivery::ProviderReport>>, AppError> {
    require_manager(&user)?;
    Ok(Json(ApiData {
        data: provider_report(&state).await?,
    }))
}

pub async fn provider_sync(
    State(state): State<AppState>,
    user: CurrentUser,
) -> Result<Json<ApiData<crate::delivery::ProviderReport>>, AppError> {
    require_manager(&user)?;
    Ok(Json(ApiData {
        data: sync_provider(&state).await?,
    }))
}

pub async fn send_message(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<Uuid>,
    Body(body): Body<SendBody>,
) -> Result<(StatusCode, Json<ApiData<SendView>>), AppError> {
    let sent = send_mail(
        &state,
        user.id,
        user.allows("mailbox.manage"),
        SendRequest {
            mailbox_id: id,
            to: body.to,
            cc: body.cc,
            bcc: body.bcc,
            subject: body.subject,
            text: body.text,
            html: body.html,
            link: None,
            draft_id: body.draft_id,
            group_ids: body.group_ids,
        },
    )
    .await?;
    Ok((
        StatusCode::CREATED,
        Json(ApiData {
            data: SendView {
                id: sent.id,
                status: sent.status,
                provider_message_id: sent.provider_message_id,
            },
        }),
    ))
}

fn require_manager(user: &CurrentUser) -> Result<(), AppError> {
    if user.allows("mailbox.manage") {
        Ok(())
    } else {
        Err(AppError::Forbidden(
            "Provider settings require mailbox management.".into(),
        ))
    }
}
