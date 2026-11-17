use axum::Json;
use axum::extract::State;
use axum::response::IntoResponse;
use serde::Serialize;

use crate::api::response::ApiData;
use crate::state::AppState;

#[derive(Serialize)]
struct BrandView {
    name: String,
    company: String,
    domain: String,
    public_url: String,
    logo_url: String,
}

pub async fn brand(State(state): State<AppState>) -> impl IntoResponse {
    let brand = &state.config.brand;
    Json(ApiData {
        data: BrandView {
            name: brand.name.clone(),
            company: brand.company.clone(),
            domain: state.config.resend.sending_domain.clone(),
            public_url: brand.public_url.clone().unwrap_or_default(),
            logo_url: brand.logo_url.clone().unwrap_or_default(),
        },
    })
}
