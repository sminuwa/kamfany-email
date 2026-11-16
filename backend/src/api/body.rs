use axum::Json;
use axum::extract::{FromRequest, Request};
use serde::de::DeserializeOwned;

use crate::api::errors::AppError;

pub struct Body<T>(pub T);

impl<S, T> FromRequest<S> for Body<T>
where
    S: Send + Sync,
    T: DeserializeOwned + 'static,
{
    type Rejection = AppError;

    async fn from_request(request: Request, state: &S) -> Result<Self, Self::Rejection> {
        match Json::<T>::from_request(request, state).await {
            Ok(Json(value)) => Ok(Self(value)),
            Err(_) => Err(AppError::Validation(
                "The request body is not valid.".into(),
            )),
        }
    }
}
