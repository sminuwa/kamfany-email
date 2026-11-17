use std::time::{Duration, SystemTime, UNIX_EPOCH};

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use chrono::{DateTime, Utc};
use hmac::{Hmac, KeyInit, Mac};
use serde::Deserialize;
use serde_json::Value;
use sha2::Sha256;

use super::{
    EmailProvider, InboundEmail, OutboundEmail, OutboundReceipt, ProviderDomain, ProviderError,
    ProviderInbox, WebhookNotice,
};

const TOLERANCE_SECONDS: i64 = 5 * 60;
const ATTEMPTS: u32 = 3;

type HmacSha256 = Hmac<Sha256>;

#[derive(Clone)]
pub struct ResendClient {
    http: reqwest::Client,
    base: String,
    key: String,
}

impl std::fmt::Debug for ResendClient {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ResendClient")
            .field("base", &self.base)
            .field("api_key", &"set")
            .finish()
    }
}

impl ResendClient {
    pub fn new(base: impl Into<String>, key: impl Into<String>) -> Result<Self, ProviderError> {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(60))
            .build()
            .map_err(|_| ProviderError::Unexpected)?;
        Ok(Self {
            http,
            base: base.into().trim_end_matches('/').to_string(),
            key: key.into(),
        })
    }
}

impl EmailProvider for ResendClient {
    async fn send_email(&self, email: &OutboundEmail) -> Result<OutboundReceipt, ProviderError> {
        let body = SendPayload {
            from: from_header(email),
            to: email.to.clone(),
            cc: email.cc.clone(),
            bcc: email.bcc.clone(),
            subject: email.subject.clone(),
            text: email.text.clone(),
            html: email.html.clone(),
            headers: email
                .headers
                .iter()
                .map(|(name, value)| (name.clone(), value.clone()))
                .collect(),
            attachments: email
                .attachments
                .iter()
                .map(|file| SendAttachment {
                    filename: file.filename.clone(),
                    content: STANDARD.encode(&file.content),
                    content_type: file.content_type.clone(),
                })
                .collect(),
        };
        let response = self
            .request(
                reqwest::Method::POST,
                "/emails",
                Some(&body),
                Some(&email.idempotency_key),
            )
            .await?;
        let parsed: SendResponse = parse_json(&response)?;
        if parsed.id.is_empty() {
            return Err(ProviderError::Unexpected);
        }
        Ok(OutboundReceipt {
            provider_message_id: parsed.id,
        })
    }

    async fn get_received_email(
        &self,
        provider_message_id: &str,
    ) -> Result<InboundEmail, ProviderError> {
        let path = format!("/emails/receiving/{provider_message_id}");
        let response = self
            .request(reqwest::Method::GET, &path, None::<&()>, None)
            .await?;
        let parsed: ReceivedPayload = parse_json(&response)?;
        Ok(inbound_from(parsed))
    }

    async fn list_inboxes(&self) -> Result<Vec<ProviderInbox>, ProviderError> {
        let response = self
            .request(
                reqwest::Method::GET,
                "/inboxes?limit=100",
                None::<&()>,
                None,
            )
            .await?;
        let parsed: List<InboxPayload> = parse_json(&response)?;
        Ok(parsed
            .data
            .into_iter()
            .map(|inbox| ProviderInbox {
                id: inbox.id,
                address: inbox.email_address,
            })
            .collect())
    }

    async fn list_domains(&self) -> Result<Vec<ProviderDomain>, ProviderError> {
        let response = self
            .request(reqwest::Method::GET, "/domains", None::<&()>, None)
            .await?;
        let parsed: List<DomainPayload> = parse_json(&response)?;
        Ok(parsed.data.into_iter().map(domain_from).collect())
    }
}

impl ResendClient {
    pub async fn list_received_attachments(
        &self,
        email_id: &str,
    ) -> Result<Vec<super::RemoteAttachment>, ProviderError> {
        let path = format!("/emails/receiving/{email_id}/attachments");
        let response = self
            .request(reqwest::Method::GET, &path, None::<&()>, None)
            .await?;
        let parsed: List<ReceivedFile> = parse_json(&response)?;
        Ok(parsed.data.into_iter().map(remote_file).collect())
    }

    pub async fn download(&self, url: &str) -> Result<Vec<u8>, ProviderError> {
        if !(url.starts_with("https://")
            || url.starts_with("http://127.0.0.1")
            || url.starts_with("http://localhost"))
        {
            return Err(ProviderError::Unexpected);
        }
        let response = self.http.get(url).send().await.map_err(|error| {
            tracing::warn!(%error, "attachment download failed");
            ProviderError::Temporary { status: 0 }
        })?;
        let status = response.status().as_u16();
        if status == 408 || status == 429 || status >= 500 {
            return Err(ProviderError::Temporary { status });
        }
        if !(200..300).contains(&status) {
            return Err(ProviderError::Rejected {
                status,
                message: "The attachment could not be downloaded.".into(),
            });
        }
        let bytes = response
            .bytes()
            .await
            .map_err(|_| ProviderError::Temporary { status })?;
        Ok(bytes.to_vec())
    }

    async fn request<T: serde::Serialize>(
        &self,
        method: reqwest::Method,
        path: &str,
        body: Option<&T>,
        idempotency_key: Option<&str>,
    ) -> Result<Vec<u8>, ProviderError> {
        let mut last = ProviderError::Temporary { status: 0 };
        for attempt in 0..ATTEMPTS {
            match self
                .request_once(method.clone(), path, body, idempotency_key)
                .await
            {
                Ok(bytes) => return Ok(bytes),
                Err(error @ ProviderError::Temporary { .. }) => {
                    last = error;
                    if attempt + 1 < ATTEMPTS {
                        tokio::time::sleep(Duration::from_millis(200 * u64::from(attempt + 1)))
                            .await;
                    }
                }
                Err(error) => return Err(error),
            }
        }
        Err(last)
    }

    async fn request_once<T: serde::Serialize>(
        &self,
        method: reqwest::Method,
        path: &str,
        body: Option<&T>,
        idempotency_key: Option<&str>,
    ) -> Result<Vec<u8>, ProviderError> {
        let mut request = self
            .http
            .request(method, format!("{}{path}", self.base))
            .header(
                reqwest::header::AUTHORIZATION,
                format!("Bearer {}", self.key),
            )
            .header(reqwest::header::ACCEPT, "application/json")
            .header(reqwest::header::USER_AGENT, "mail-desk/0.1");
        if let Some(key) = idempotency_key {
            request = request.header("Idempotency-Key", key);
        }
        if let Some(body) = body {
            request = request.json(body);
        }
        let response = request.send().await.map_err(|error| {
            tracing::warn!(%error, "resend request failed");
            ProviderError::Temporary { status: 0 }
        })?;
        let status = response.status().as_u16();
        let bytes = response
            .bytes()
            .await
            .map_err(|_| ProviderError::Temporary { status })?;
        if (200..300).contains(&status) {
            return Ok(bytes.to_vec());
        }
        if status == 408 || status == 429 || status >= 500 {
            tracing::warn!(status, "resend temporary failure");
            return Err(ProviderError::Temporary { status });
        }
        let message = serde_json::from_slice::<ErrorPayload>(&bytes)
            .ok()
            .and_then(|payload| payload.message)
            .filter(|message| !message.is_empty())
            .unwrap_or_else(|| "Resend rejected the request.".to_string());
        tracing::warn!(status, "resend rejected the request");
        Err(ProviderError::Rejected { status, message })
    }
}

pub fn verify_signature(
    secret: &str,
    id: &str,
    timestamp: &str,
    signature: &str,
    body: &[u8],
    now_unix: i64,
) -> bool {
    if id.is_empty() || id.contains(['.', '\n', '\r']) || timestamp.contains(['.', '\n', '\r']) {
        return false;
    }
    let Ok(sent) = timestamp.parse::<i64>() else {
        return false;
    };
    if (now_unix - sent).abs() > TOLERANCE_SECONDS {
        return false;
    }
    let Some(secret_bytes) = decode_secret(secret) else {
        return false;
    };
    let mut signed = Vec::with_capacity(id.len() + timestamp.len() + body.len() + 2);
    signed.extend_from_slice(id.as_bytes());
    signed.push(b'.');
    signed.extend_from_slice(timestamp.as_bytes());
    signed.push(b'.');
    signed.extend_from_slice(body);
    signature.split_whitespace().any(|part| {
        let Some(encoded) = part.strip_prefix("v1,") else {
            return false;
        };
        mac_matches(&secret_bytes, &signed, encoded)
    })
}

pub fn parse_webhook(payload: &Value) -> Result<WebhookNotice, ProviderError> {
    let event_type = payload
        .get("type")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or(ProviderError::Unexpected)?
        .to_string();
    let data = payload.get("data").cloned().unwrap_or(Value::Null);
    let email_id = data
        .get("email_id")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .map(str::to_string);
    let mut recipients = Vec::new();
    for key in ["to", "cc", "bcc", "received_for"] {
        push_addresses(&data, key, &mut recipients);
    }
    Ok(WebhookNotice {
        event_type,
        email_id,
        recipients,
    })
}

fn decode_secret(secret: &str) -> Option<Vec<u8>> {
    let encoded = secret.strip_prefix("whsec_").unwrap_or(secret);
    STANDARD.decode(encoded).ok()
}

fn mac_matches(secret: &[u8], signed: &[u8], encoded: &str) -> bool {
    let Ok(expected) = STANDARD.decode(encoded) else {
        return false;
    };
    let Ok(mut mac) = HmacSha256::new_from_slice(secret) else {
        return false;
    };
    mac.update(signed);
    mac.verify_slice(&expected).is_ok()
}

fn from_header(email: &OutboundEmail) -> String {
    if email.from_name.is_empty() || email.from_name.contains(['<', '>', '"', '\n', '\r', ',']) {
        email.from_address.clone()
    } else {
        format!("{} <{}>", email.from_name, email.from_address)
    }
}

fn parse_json<T: for<'de> Deserialize<'de>>(bytes: &[u8]) -> Result<T, ProviderError> {
    serde_json::from_slice(bytes).map_err(|_| ProviderError::Unexpected)
}

fn inbound_from(email: ReceivedPayload) -> InboundEmail {
    let (from_address, from_name) = split_mailbox(&email.from);
    let header = |name: &str| {
        email
            .headers
            .get(name)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
    };
    InboundEmail {
        provider_message_id: email.id,
        from_address,
        from_name,
        to: email.to,
        cc: email.cc,
        subject: email.subject.unwrap_or_default(),
        text: blank_to_none(email.text),
        html: blank_to_none(email.html),
        message_id_header: header("message-id"),
        in_reply_to: header("in-reply-to"),
        references_header: header("references"),
        received_at: email
            .created_at
            .as_deref()
            .and_then(|value| DateTime::parse_from_rfc3339(value).ok())
            .map(|value| value.with_timezone(&Utc)),
        attachment_count: email.attachments.len(),
    }
}

fn domain_from(domain: DomainPayload) -> ProviderDomain {
    ProviderDomain {
        name: domain.name,
        status: domain.status,
        sending: domain
            .capabilities
            .as_ref()
            .and_then(|item| item.sending.clone()),
        receiving: domain
            .capabilities
            .as_ref()
            .and_then(|item| item.receiving.clone()),
    }
}

fn split_mailbox(value: &str) -> (String, Option<String>) {
    let trimmed = value.trim();
    if let Some((name, rest)) = trimmed.rsplit_once('<')
        && let Some(address) = rest.strip_suffix('>')
    {
        let name = name.trim().trim_matches('"').to_string();
        return (
            address.trim().to_string(),
            (!name.is_empty()).then_some(name),
        );
    }
    (trimmed.to_string(), None)
}

fn remote_file(file: ReceivedFile) -> super::RemoteAttachment {
    super::RemoteAttachment {
        id: file.id,
        filename: file
            .filename
            .filter(|name| !name.trim().is_empty())
            .unwrap_or_else(|| "attachment".to_string()),
        content_type: file
            .content_type
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| "application/octet-stream".to_string()),
        size: file.size.unwrap_or(0),
        download_url: file.download_url.filter(|url| !url.is_empty()),
    }
}

fn blank_to_none(value: Option<String>) -> Option<String> {
    value.filter(|item| !item.trim().is_empty())
}

fn push_addresses(data: &Value, key: &str, into: &mut Vec<String>) {
    match data.get(key) {
        Some(Value::String(address)) => into.push(address.clone()),
        Some(Value::Array(addresses)) => {
            for address in addresses {
                if let Some(address) = address.as_str() {
                    into.push(address.to_string());
                }
            }
        }
        _ => {}
    }
}

#[derive(serde::Serialize)]
struct SendPayload {
    from: String,
    to: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    cc: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    bcc: Vec<String>,
    subject: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    html: Option<String>,
    #[serde(skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    headers: std::collections::BTreeMap<String, String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    attachments: Vec<SendAttachment>,
}

#[derive(serde::Serialize)]
struct SendAttachment {
    filename: String,
    content: String,
    content_type: String,
}

#[derive(Deserialize)]
struct SendResponse {
    id: String,
}

#[derive(Deserialize)]
struct ErrorPayload {
    message: Option<String>,
}

#[derive(Deserialize)]
struct List<T> {
    data: Vec<T>,
}

#[derive(Deserialize)]
struct InboxPayload {
    id: String,
    email_address: String,
}

#[derive(Deserialize)]
struct DomainPayload {
    name: String,
    status: String,
    capabilities: Option<Capabilities>,
}

#[derive(Deserialize)]
struct Capabilities {
    sending: Option<String>,
    receiving: Option<String>,
}

#[derive(Deserialize)]
struct ReceivedPayload {
    id: String,
    from: String,
    #[serde(default)]
    to: Vec<String>,
    #[serde(default)]
    cc: Vec<String>,
    subject: Option<String>,
    text: Option<String>,
    html: Option<String>,
    created_at: Option<String>,
    #[serde(default)]
    headers: serde_json::Map<String, Value>,
    #[serde(default)]
    attachments: Vec<Value>,
}

#[derive(Deserialize)]
struct ReceivedFile {
    id: String,
    filename: Option<String>,
    content_type: Option<String>,
    size: Option<i64>,
    download_url: Option<String>,
}

pub fn unix_now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::{parse_webhook, verify_signature};
    use base64::Engine;
    use base64::engine::general_purpose::STANDARD;
    use hmac::{Hmac, KeyInit, Mac};
    use sha2::Sha256;

    #[test]
    fn a_signed_webhook_verifies_and_a_changed_body_does_not() {
        let secret_bytes = b"kamfany-mail-test-secret";
        let secret = format!("whsec_{}", STANDARD.encode(secret_bytes));
        let id = "msg_2a2";
        let timestamp = "1700000000";
        let body = br#"{"type":"email.received","data":{"email_id":"em_1"}}"#;
        let mut signed = Vec::new();
        signed.extend_from_slice(id.as_bytes());
        signed.push(b'.');
        signed.extend_from_slice(timestamp.as_bytes());
        signed.push(b'.');
        signed.extend_from_slice(body);
        let mut mac = <Hmac<Sha256> as KeyInit>::new_from_slice(secret_bytes).unwrap();
        mac.update(&signed);
        let signature = format!("v1,{}", STANDARD.encode(mac.finalize().into_bytes()));

        assert!(verify_signature(
            &secret,
            id,
            timestamp,
            &signature,
            body,
            1_700_000_100
        ));
        assert!(!verify_signature(
            &secret,
            id,
            timestamp,
            &signature,
            b"{}",
            1_700_000_100
        ));
        assert!(!verify_signature(
            &secret,
            id,
            timestamp,
            &signature,
            body,
            1_700_000_400
        ));
    }

    #[test]
    fn webhook_metadata_collects_recipients_without_a_body() {
        let payload = serde_json::json!({
            "type": "email.received",
            "data": {
                "email_id": "em_1",
                "to": ["support@kamfany.com"],
                "cc": "finance@kamfany.com",
                "received_for": ["info@kamfany.com"]
            }
        });
        let notice = parse_webhook(&payload).expect("webhook");
        assert_eq!(notice.event_type, "email.received");
        assert_eq!(notice.email_id.as_deref(), Some("em_1"));
        assert_eq!(
            notice.recipients,
            vec![
                "support@kamfany.com",
                "finance@kamfany.com",
                "info@kamfany.com"
            ]
        );
    }
}
