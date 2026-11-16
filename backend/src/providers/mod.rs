//! Email provider boundary.
//!
//! Resend request and response types stay in `resend`. The rest of the API
//! uses the models in this module.

use std::future::Future;

mod resend;

pub use resend::{ResendClient, parse_webhook, unix_now, verify_signature};

use chrono::{DateTime, Utc};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutboundEmail {
    pub from_name: String,
    pub from_address: String,
    pub to: Vec<String>,
    pub cc: Vec<String>,
    pub bcc: Vec<String>,
    pub subject: String,
    pub text: Option<String>,
    pub html: Option<String>,
    pub idempotency_key: String,
    /// RFC 5322 headers Resend accepts on `POST /emails`, such as In-Reply-To.
    pub headers: Vec<(String, String)>,
    pub attachments: Vec<OutboundAttachment>,
}

#[derive(Clone, PartialEq, Eq)]
pub struct OutboundAttachment {
    pub filename: String,
    pub content_type: String,
    pub content: Vec<u8>,
}

impl std::fmt::Debug for OutboundAttachment {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("OutboundAttachment")
            .field("filename", &self.filename)
            .field("content_type", &self.content_type)
            .field("bytes", &self.content.len())
            .finish()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutboundReceipt {
    pub provider_message_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InboundEmail {
    pub provider_message_id: String,
    pub from_address: String,
    pub from_name: Option<String>,
    pub to: Vec<String>,
    pub cc: Vec<String>,
    pub subject: String,
    pub text: Option<String>,
    pub html: Option<String>,
    pub message_id_header: Option<String>,
    pub in_reply_to: Option<String>,
    pub references_header: Option<String>,
    pub received_at: Option<DateTime<Utc>>,
    pub attachment_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteAttachment {
    pub id: String,
    pub filename: String,
    pub content_type: String,
    pub size: i64,
    pub download_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderInbox {
    pub id: String,
    pub address: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderDomain {
    pub name: String,
    pub status: String,
    pub sending: Option<String>,
    pub receiving: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WebhookNotice {
    pub event_type: String,
    pub email_id: Option<String>,
    pub recipients: Vec<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum ProviderError {
    #[error("Resend is not configured.")]
    NotConfigured,
    #[error("{message}")]
    Rejected { status: u16, message: String },
    #[error("The email provider is unavailable.")]
    Temporary { status: u16 },
    #[error("The email provider returned an unexpected response.")]
    Unexpected,
}

pub trait EmailProvider: Send + Sync {
    fn send_email(
        &self,
        email: &OutboundEmail,
    ) -> impl Future<Output = Result<OutboundReceipt, ProviderError>> + Send;

    fn get_received_email(
        &self,
        provider_message_id: &str,
    ) -> impl Future<Output = Result<InboundEmail, ProviderError>> + Send;

    fn list_inboxes(
        &self,
    ) -> impl Future<Output = Result<Vec<ProviderInbox>, ProviderError>> + Send;

    fn list_domains(
        &self,
    ) -> impl Future<Output = Result<Vec<ProviderDomain>, ProviderError>> + Send;
}
