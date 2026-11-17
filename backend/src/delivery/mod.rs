//! Outbound mail, inbound storage, and the durable provider queue.
//!
//! Provider HTTP types stay in `providers`. This module stores KAMFANY records.

mod files;
mod ingest;
mod jobs;
mod send;
mod sync;

pub(crate) use files::{purge_message_files, store_received_files};

pub use ingest::store_inbound;
pub use jobs::{accept_webhook, process_due, run_jobs};
pub use send::{
    GroupDelivery, GroupFile, SendLink, SendRequest, SentMessage, deliver_group_mail, send_mail,
};
pub use sync::{ProviderReport, provider_report, sync_provider};

use crate::api::AppError;
use crate::providers::ProviderError;

pub fn provider_error(error: ProviderError) -> AppError {
    match error {
        ProviderError::NotConfigured => AppError::Unavailable("Resend is not configured.".into()),
        ProviderError::Rejected { message, .. } => AppError::Provider(message),
        ProviderError::Temporary { .. } => {
            AppError::Unavailable("The email provider is unavailable. Try again.".into())
        }
        ProviderError::Unexpected => {
            tracing::error!("resend returned an unexpected response");
            AppError::Internal
        }
    }
}

fn preview(text: &str) -> Option<String> {
    let flat = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let shortened: String = flat.chars().take(140).collect();
    (!shortened.is_empty()).then_some(shortened)
}
