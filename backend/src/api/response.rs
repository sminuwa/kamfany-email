use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct ApiData<T> {
    pub data: T,
}
