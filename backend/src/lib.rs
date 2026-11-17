//! Company mail API. The install name, company, logo, and sending domain come from configuration.
//!
//! Browsers talk only to this service. Provider credentials and Resend payloads
//! stay inside `providers`.

pub mod api;
pub mod auth;
pub mod bootstrap;
pub mod compose;
pub mod config;
pub mod contacts;
pub mod database;
pub mod delivery;
pub mod limit;
pub mod live;
pub mod messages;
pub mod providers;
mod records;
pub mod scan;
pub mod state;
pub mod storage;

pub use api::router;
pub use config::AppConfig;
pub use state::{AppState, build_state};
