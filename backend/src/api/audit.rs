use axum::Json;
use axum::extract::{Query, State};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::api::errors::AppError;
use crate::api::response::ApiData;
use crate::auth::{CurrentUser, pool_of};
use crate::state::AppState;

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct AuditItem {
    pub id: Uuid,
    pub action: String,
    pub entity_type: Option<String>,
    pub entity_id: Option<Uuid>,
    pub actor_email: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Deserialize)]
pub struct AuditQuery {
    limit: Option<i64>,
}

pub async fn list_audit(
    State(state): State<AppState>,
    user: CurrentUser,
    Query(query): Query<AuditQuery>,
) -> Result<Json<ApiData<Vec<AuditItem>>>, AppError> {
    if !user.allows("audit.read") {
        return Err(AppError::Forbidden(
            "Audit review requires the audit permission.".into(),
        ));
    }
    let pool = pool_of(&state)?;
    let limit = query.limit.unwrap_or(50).clamp(1, 100);
    let rows = sqlx::query_as::<_, AuditItem>(
        r#"
        SELECT a.id, a.action, a.entity_type, a.entity_id, a.created_at,
               u.email::text AS actor_email
        FROM audit_logs a
        LEFT JOIN users u ON u.id = a.actor_user_id
        ORDER BY a.created_at DESC
        LIMIT $1
        "#,
    )
    .bind(limit)
    .fetch_all(pool)
    .await?;
    Ok(Json(ApiData { data: rows }))
}
