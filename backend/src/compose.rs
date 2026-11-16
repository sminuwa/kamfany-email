//! Outbound HTML and attachment names.
//!
//! The browser's editor is not trusted. HTML is reduced to the formatting the
//! composer offers, and file names cannot carry a path.

use std::collections::HashSet;

use ammonia::UrlRelative;

use crate::api::AppError;

pub const MAX_FILES: usize = 10;
pub const MAX_FILE_BYTES: usize = 10 * 1024 * 1024;
pub const MAX_TOTAL_BYTES: i64 = 25 * 1024 * 1024;

const BLOCKED_EXTENSIONS: &[&str] = &[
    "apk", "bat", "cmd", "com", "cpl", "dll", "dmg", "exe", "hta", "iso", "jar", "js", "lnk",
    "msi", "ps1", "scr", "vbs",
];

pub fn message_content(
    text: Option<String>,
    html: Option<String>,
) -> Result<(Option<String>, Option<String>), AppError> {
    let html = html
        .map(|value| sanitize_html(&value))
        .filter(|value| !value.trim().is_empty() && !html_to_text(value).trim().is_empty());
    let html = match html {
        Some(value) if value.chars().count() > 200_000 => {
            return Err(AppError::Validation("The message body is too long.".into()));
        }
        other => other,
    };
    let text = match text {
        Some(value) if !value.trim().is_empty() => Some(value.trim().to_string()),
        _ => html.as_deref().map(html_to_text).filter(|value| !value.is_empty()),
    };
    if text.as_ref().is_some_and(|value| value.chars().count() > 200_000) {
        return Err(AppError::Validation("The message body is too long.".into()));
    }
    Ok((text, html))
}

pub fn sanitize_html(value: &str) -> String {
    let mut tags = HashSet::new();
    for tag in [
        "a", "blockquote", "br", "div", "em", "h1", "h2", "h3", "i", "li", "ol", "p", "span",
        "strong", "u", "ul", "b",
    ] {
        tags.insert(tag);
    }
    let mut schemes = HashSet::new();
    schemes.insert("http");
    schemes.insert("https");
    schemes.insert("mailto");
    let mut builder = ammonia::Builder::new();
    builder
        .tags(tags)
        .url_schemes(schemes)
        .url_relative(UrlRelative::Deny)
        .link_rel(Some("noopener noreferrer"))
        .add_tag_attributes("div", &["style", "data-signature"])
        .add_tag_attributes("p", &["style"])
        .add_tag_attributes("h1", &["style"])
        .add_tag_attributes("h2", &["style"])
        .add_tag_attributes("h3", &["style"])
        .add_tag_attributes("span", &["style"])
        .add_tag_attributes("li", &["style"])
        .attribute_filter(|_element, attribute, value| {
            let keep = match attribute {
                "href" => href_is_safe(value),
                "style" => style_is_safe(value),
                "data-signature" => value == "1",
                "rel" => value == "noopener noreferrer",
                _ => false,
            };
            keep.then_some(std::borrow::Cow::Borrowed(value))
        });
    builder.clean(value).to_string()
}

pub fn html_to_text(value: &str) -> String {
    let broken = value
        .replace("<br>", "\n")
        .replace("<br/>", "\n")
        .replace("<br />", "\n")
        .replace("</p>", "\n")
        .replace("</div>", "\n")
        .replace("</li>", "\n")
        .replace("</h1>", "\n")
        .replace("</h2>", "\n")
        .replace("</h3>", "\n")
        .replace("</blockquote>", "\n");
    let mut text = String::new();
    let mut tag = false;
    for character in broken.chars() {
        match character {
            '<' => tag = true,
            '>' => tag = false,
            _ if !tag => text.push(character),
            _ => {}
        }
    }
    let text = text
        .replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'");
    let mut collapsed = String::new();
    let mut blank = 0;
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            blank += 1;
            if blank <= 2 {
                collapsed.push('\n');
            }
            continue;
        }
        blank = 0;
        if !collapsed.is_empty() && !collapsed.ends_with('\n') {
            collapsed.push('\n');
        }
        collapsed.push_str(line);
        collapsed.push('\n');
    }
    collapsed.trim().to_string()
}

pub fn safe_filename(value: &str) -> Result<String, AppError> {
    let base = value
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(value)
        .trim()
        .trim_matches('.');
    let mut name = String::new();
    for character in base.chars() {
        if character.is_control() || matches!(character, '<' | '>' | ':' | '"' | '|' | '?' | '*') {
            continue;
        }
        name.push(character);
        if name.chars().count() == 180 {
            break;
        }
    }
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err(AppError::Validation(
            "Choose a file with a name.".into(),
        ));
    }
    Ok(name)
}

pub fn outbound_filename(value: &str) -> Result<String, AppError> {
    let name = safe_filename(value)?;
    let extension = name
        .rsplit_once('.')
        .map(|(_, extension)| extension.to_ascii_lowercase())
        .unwrap_or_default();
    if BLOCKED_EXTENSIONS.contains(&extension.as_str()) {
        return Err(AppError::Validation(format!(
            "{name} is not an allowed attachment type."
        )));
    }
    Ok(name)
}

pub fn content_type(value: &str) -> String {
    let value = value.trim().to_ascii_lowercase();
    if (1..80).contains(&value.len())
        && value.contains('/')
        && value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '/' | '+' | '-' | '.'))
    {
        return value;
    }
    "application/octet-stream".to_string()
}

pub fn inline_preview(content_type: &str) -> bool {
    matches!(
        content_type,
        "image/png" | "image/jpeg" | "image/gif" | "image/webp"
    )
}

fn style_is_safe(value: &str) -> bool {
    value.split(';').all(|part| {
        let part = part.trim();
        if part.is_empty() {
            return true;
        }
        let Some((name, raw)) = part.split_once(':') else {
            return false;
        };
        match name.trim() {
            "color" => color_is_safe(raw.trim()),
            "text-align" => matches!(raw.trim(), "left" | "right" | "center" | "justify"),
            _ => false,
        }
    })
}

fn href_is_safe(value: &str) -> bool {
    let value = value.trim();
    value.starts_with("https://")
        || value.starts_with("http://")
        || value.starts_with("mailto:")
}

fn color_is_safe(value: &str) -> bool {
    let hex = value.strip_prefix('#').unwrap_or("");
    matches!(hex.len(), 3 | 6 | 8) && hex.chars().all(|character| character.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::{html_to_text, outbound_filename, sanitize_html};

    #[test]
    fn html_keeps_formatting_and_drops_scripts() {
        let clean = sanitize_html(
            r#"<p style="text-align: left">Hello <strong>there</strong></p><script>alert(1)</script><a href="javascript:alert(1)">x</a>"#,
        );
        assert!(clean.contains("<strong>"));
        assert!(!clean.to_ascii_lowercase().contains("script"));
        assert!(!clean.to_ascii_lowercase().contains("javascript"));
        assert_eq!(html_to_text(&clean), "Hello there\nx");
    }

    #[test]
    fn executable_names_are_refused() {
        assert!(outbound_filename("notes/invoice.pdf").is_ok());
        assert!(outbound_filename("setup.exe").is_err());
        assert!(outbound_filename("..\\payload.bat").is_err());
    }
}
