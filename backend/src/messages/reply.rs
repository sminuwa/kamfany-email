use uuid::Uuid;

use crate::api::AppError;
use crate::auth::CurrentUser;
use crate::delivery::{SendLink, SendRequest, SentMessage, send_mail};
use crate::records::header_tokens;
use crate::state::AppState;

use super::access::{message_mailbox, require_reader};

pub struct ComposeFields {
    pub to: Option<Vec<String>>,
    pub cc: Option<Vec<String>>,
    pub bcc: Option<Vec<String>>,
    pub subject: Option<String>,
    pub text: Option<String>,
    pub html: Option<String>,
    pub draft_id: Option<Uuid>,
}

pub async fn reply_message(
    state: &AppState,
    user: &CurrentUser,
    message_id: Uuid,
    all: bool,
    fields: ComposeFields,
) -> Result<SentMessage, AppError> {
    let source = load_source(state, user, message_id).await?;
    let (default_to, default_cc) = reply_recipients(
        &source.from_address,
        &source.to,
        &source.cc,
        &source.mailbox_address,
        all,
    );
    let to = fields.to.unwrap_or(default_to);
    let cc = fields.cc.unwrap_or(default_cc);
    let subject = fields
        .subject
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| prefixed(&source.subject, "Re:"));
    let text = fields
        .text
        .or_else(|| Some(quote(&source.from_address, &source.body_text)));
    send_linked(
        state,
        user,
        &source,
        LinkedMail {
            to,
            cc,
            bcc: fields.bcc.unwrap_or_default(),
            subject,
            text,
            html: fields.html,
            draft_id: fields.draft_id,
            fresh: false,
        },
    )
    .await
}

pub async fn forward_message(
    state: &AppState,
    user: &CurrentUser,
    message_id: Uuid,
    fields: ComposeFields,
) -> Result<SentMessage, AppError> {
    let source = load_source(state, user, message_id).await?;
    let subject = fields
        .subject
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| prefixed(&source.subject, "Fwd:"));
    let text = fields
        .text
        .or_else(|| Some(quote(&source.from_address, &source.body_text)));
    send_linked(
        state,
        user,
        &source,
        LinkedMail {
            to: fields.to.unwrap_or_default(),
            cc: fields.cc.unwrap_or_default(),
            bcc: fields.bcc.unwrap_or_default(),
            subject,
            text,
            html: fields.html,
            draft_id: fields.draft_id,
            fresh: true,
        },
    )
    .await
}

struct LinkedMail {
    to: Vec<String>,
    cc: Vec<String>,
    bcc: Vec<String>,
    subject: String,
    text: Option<String>,
    html: Option<String>,
    draft_id: Option<Uuid>,
    fresh: bool,
}

pub fn reply_recipients(
    from: &str,
    to: &[String],
    cc: &[String],
    mailbox: &str,
    all: bool,
) -> (Vec<String>, Vec<String>) {
    let from_self = from.eq_ignore_ascii_case(mailbox);
    let mut primary: Vec<String> = if from_self {
        to.iter()
            .filter(|address| !address.eq_ignore_ascii_case(mailbox))
            .cloned()
            .collect()
    } else {
        vec![from.to_string()]
    };
    dedup_addresses(&mut primary);
    if !all {
        return (primary, Vec::new());
    }
    let mut seen: Vec<String> = primary
        .iter()
        .map(|address| address.to_ascii_lowercase())
        .collect();
    seen.push(mailbox.to_ascii_lowercase());
    let others: Vec<String> = if from_self {
        cc.to_vec()
    } else {
        to.iter().chain(cc.iter()).cloned().collect()
    };
    let mut copied = Vec::new();
    for address in others {
        let key = address.to_ascii_lowercase();
        if seen.iter().any(|item| item == &key) {
            continue;
        }
        seen.push(key);
        copied.push(address);
    }
    (primary, copied)
}

fn dedup_addresses(addresses: &mut Vec<String>) {
    let mut seen = Vec::new();
    addresses.retain(|address| {
        let key = address.to_ascii_lowercase();
        if seen.contains(&key) {
            return false;
        }
        seen.push(key);
        true
    });
}

fn prefixed(subject: &str, prefix: &str) -> String {
    let trimmed = subject.trim();
    if trimmed
        .to_ascii_lowercase()
        .starts_with(&prefix.to_ascii_lowercase())
    {
        trimmed.to_string()
    } else {
        format!("{prefix} {trimmed}")
    }
}

fn quote(from: &str, body: &str) -> String {
    let excerpt: String = body.chars().take(20_000).collect();
    format!("\n\n{from} wrote:\n\n{excerpt}")
}

struct Source {
    mailbox_id: Uuid,
    mailbox_address: String,
    thread_id: Option<Uuid>,
    subject: String,
    from_address: String,
    message_id_header: Option<String>,
    references_header: Option<String>,
    body_text: String,
    to: Vec<String>,
    cc: Vec<String>,
}

async fn load_source(
    state: &AppState,
    user: &CurrentUser,
    message_id: Uuid,
) -> Result<Source, AppError> {
    let pool = crate::auth::pool_of(state)?;
    let mailbox_id = message_mailbox(pool, message_id).await?;
    let grant = require_reader(pool, mailbox_id, user)
        .await
        .map_err(|error| match error {
            AppError::Forbidden(_) => AppError::NotFound,
            other => other,
        })?;
    if !grant.can_send {
        return Err(AppError::Forbidden(
            "Sending is not granted for this mailbox.".into(),
        ));
    }
    let row = sqlx::query_as::<_, SourceRow>(
        r#"
        SELECT m.thread_id,
               m.subject,
               m.from_address::text AS from_address,
               m.message_id_header,
               m.references_header,
               COALESCE(m.body_text, m.preview, '') AS body_text,
               boxes.address::text AS mailbox_address
        FROM messages m
        JOIN mailboxes boxes ON boxes.id = m.mailbox_id
        WHERE m.id = $1
        "#,
    )
    .bind(message_id)
    .fetch_optional(pool)
    .await?
    .ok_or(AppError::NotFound)?;
    let people = sqlx::query_as::<_, AddressRow>(
        "SELECT kind, address::text AS address FROM recipients WHERE message_id = $1",
    )
    .bind(message_id)
    .fetch_all(pool)
    .await?;
    let parent_header = row.message_id_header.clone().unwrap_or_else(|| {
        let domain = row
            .mailbox_address
            .split_once('@')
            .map(|(_, domain)| domain)
            .unwrap_or(state.config.resend.sending_domain.as_str());
        format!("<{message_id}@{domain}>")
    });
    if row.message_id_header.is_none() {
        sqlx::query(
            "UPDATE messages SET message_id_header = $2 WHERE id = $1 AND message_id_header IS NULL",
        )
        .bind(message_id)
        .bind(&parent_header)
        .execute(pool)
        .await?;
    }
    Ok(Source {
        mailbox_id,
        mailbox_address: row.mailbox_address,
        thread_id: row.thread_id,
        subject: row.subject,
        from_address: row.from_address,
        message_id_header: Some(parent_header),
        references_header: row.references_header,
        body_text: row.body_text,
        to: people
            .iter()
            .filter(|item| item.kind == "to")
            .map(|item| item.address.clone())
            .collect(),
        cc: people
            .iter()
            .filter(|item| item.kind == "cc")
            .map(|item| item.address.clone())
            .collect(),
    })
}

async fn send_linked(
    state: &AppState,
    user: &CurrentUser,
    source: &Source,
    mail: LinkedMail,
) -> Result<SentMessage, AppError> {
    let parent = source.message_id_header.clone().unwrap_or_default();
    let mut references = header_tokens(None, source.references_header.as_deref());
    if !parent.is_empty() && !references.iter().any(|token| token == &parent) {
        references.push(parent.clone());
    }
    send_mail(
        state,
        user.id,
        user.allows("mailbox.manage"),
        SendRequest {
            mailbox_id: source.mailbox_id,
            to: mail.to,
            cc: mail.cc,
            bcc: mail.bcc,
            subject: mail.subject,
            text: mail.text,
            html: mail.html,
            draft_id: mail.draft_id,
            link: Some(SendLink {
                thread_id: if mail.fresh { None } else { source.thread_id },
                in_reply_to: (!parent.is_empty()).then_some(parent),
                references_header: (!references.is_empty()).then(|| references.join(" ")),
                fresh: mail.fresh,
            }),
        },
    )
    .await
}

#[derive(sqlx::FromRow)]
struct SourceRow {
    thread_id: Option<Uuid>,
    subject: String,
    from_address: String,
    message_id_header: Option<String>,
    references_header: Option<String>,
    body_text: String,
    mailbox_address: String,
}

#[derive(sqlx::FromRow)]
struct AddressRow {
    kind: String,
    address: String,
}

#[cfg(test)]
mod tests {
    use super::{prefixed, reply_recipients};

    #[test]
    fn reply_all_keeps_the_sender_and_drops_self() {
        let (to, cc) = reply_recipients(
            "amaka@northwind.example",
            &[
                "support@kamfany.com".to_string(),
                "info@kamfany.com".to_string(),
            ],
            &["finance@kamfany.com".to_string()],
            "support@kamfany.com",
            true,
        );
        assert_eq!(to, vec!["amaka@northwind.example".to_string()]);
        assert_eq!(
            cc,
            vec![
                "info@kamfany.com".to_string(),
                "finance@kamfany.com".to_string()
            ]
        );
    }

    #[test]
    fn a_reply_to_sent_mail_uses_the_recipients() {
        let (to, cc) = reply_recipients(
            "support@kamfany.com",
            &["info@kamfany.com".to_string()],
            &["finance@kamfany.com".to_string()],
            "support@kamfany.com",
            true,
        );
        assert_eq!(to, vec!["info@kamfany.com".to_string()]);
        assert_eq!(cc, vec!["finance@kamfany.com".to_string()]);
        assert_eq!(prefixed("Re: Invoice", "Re:"), "Re: Invoice");
        assert_eq!(prefixed("Invoice", "Fwd:"), "Fwd: Invoice");
    }
}
