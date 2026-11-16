use axum::Json;
use axum::extract::{Query, State};
use serde::Deserialize;
use uuid::Uuid;

use crate::api::errors::AppError;
use crate::api::response::ApiData;
use crate::auth::CurrentUser;
use crate::auth::pool_of;
use crate::messages::listing::MessagePage;
use crate::messages::search::{self, SearchQuery};
use crate::state::AppState;

#[derive(Deserialize)]
pub struct SearchParams {
    q: Option<String>,
    from: Option<String>,
    to: Option<String>,
    subject: Option<String>,
    after: Option<String>,
    before: Option<String>,
    has_attachment: Option<bool>,
    unread: Option<bool>,
    important: Option<bool>,
    mailbox_id: Option<Uuid>,
    folder: Option<String>,
    label_id: Option<Uuid>,
    cursor: Option<String>,
}

pub async fn search(
    State(state): State<AppState>,
    user: CurrentUser,
    Query(params): Query<SearchParams>,
) -> Result<Json<ApiData<MessagePage>>, AppError> {
    let pool = pool_of(&state)?;
    let page = crate::messages::search_messages(pool, &user, query_of(params)?).await?;
    Ok(Json(ApiData { data: page }))
}

fn query_of(params: SearchParams) -> Result<SearchQuery, AppError> {
    Ok(SearchQuery {
        q: optional(params.q, search::plain_query)?,
        from: optional(params.from, search::like_contains)?,
        to: optional(params.to, search::like_contains)?,
        subject: optional(params.subject, search::like_contains)?,
        after: optional(params.after, |value| search::parse_day(value, false))?,
        before: optional(params.before, |value| search::parse_day(value, true))?,
        has_attachment: params.has_attachment,
        unread: params.unread,
        important: params.important,
        mailbox_id: params.mailbox_id,
        folder: params.folder.filter(|value| !value.trim().is_empty()),
        label_id: params.label_id,
        cursor: params.cursor.filter(|value| !value.trim().is_empty()),
    })
}

fn optional<T>(
    value: Option<String>,
    parse: impl FnOnce(&str) -> Result<T, AppError>,
) -> Result<Option<T>, AppError> {
    match value
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        Some(value) => Ok(Some(parse(value)?)),
        None => Ok(None),
    }
}
