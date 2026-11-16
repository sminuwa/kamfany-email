use axum::http::{HeaderMap, HeaderValue, header};
use sha2::{Digest, Sha256};

use crate::config::Environment;

pub const SESSION_COOKIE: &str = "kamfany_session";
const SESSION_SECONDS: u64 = 60 * 60 * 24 * 7;

pub fn new_token() -> String {
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes).expect("system random");
    encode_hex(&bytes)
}

pub fn hash_token(token: &str) -> String {
    encode_hex(&Sha256::digest(token.as_bytes()))
}

pub fn read_cookie(headers: &HeaderMap, name: &str) -> Option<String> {
    let value = headers.get(header::COOKIE)?.to_str().ok()?;
    for part in value.split(';') {
        let (key, raw) = part.trim().split_once('=')?;
        if key == name {
            let token = raw.trim();
            if token.len() == 64 && token.bytes().all(|byte| byte.is_ascii_hexdigit()) {
                return Some(token.to_ascii_lowercase());
            }
            return None;
        }
    }
    None
}

pub fn session_cookie(token: &str, env: Environment) -> HeaderValue {
    cookie_value(token, env, SESSION_SECONDS)
}

pub fn clear_session_cookie(env: Environment) -> HeaderValue {
    cookie_value("", env, 0)
}

fn cookie_value(token: &str, env: Environment, max_age: u64) -> HeaderValue {
    let secure = if env == Environment::Production {
        "; Secure"
    } else {
        ""
    };
    HeaderValue::from_str(&format!(
        "{SESSION_COOKIE}={token}; HttpOnly; SameSite=Lax; Path=/; Max-Age={max_age}{secure}"
    ))
    .expect("session cookie is ascii")
}

fn encode_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0xf) as usize] as char);
    }
    out
}

pub fn browser_request_allowed(
    origin: Option<&str>,
    marker: Option<&str>,
    allowed_origins: &[String],
) -> bool {
    let origin_ok = origin.is_some_and(|value| allowed_origins.iter().any(|item| item == value));
    origin_ok && marker == Some("1")
}

#[cfg(test)]
mod tests {
    use super::{browser_request_allowed, hash_token, new_token, read_cookie};
    use axum::http::{HeaderMap, HeaderValue};

    #[test]
    fn session_tokens_are_hashed_before_storage() {
        let token = new_token();
        assert_eq!(token.len(), 64);
        assert_ne!(hash_token(&token), token);
        assert_eq!(hash_token(&token), hash_token(&token));
    }

    #[test]
    fn cookies_ignore_unexpected_token_shapes() {
        let mut headers = HeaderMap::new();
        headers.insert(
            axum::http::header::COOKIE,
            HeaderValue::from_static("kamfany_session=short; other=1"),
        );
        assert!(read_cookie(&headers, "kamfany_session").is_none());
    }

    #[test]
    fn browser_mutations_need_an_allowed_origin_and_the_desk_header() {
        let allowed = vec!["http://localhost:5173".to_string()];
        assert!(browser_request_allowed(
            Some("http://localhost:5173"),
            Some("1"),
            &allowed
        ));
        assert!(!browser_request_allowed(
            Some("https://evil.example"),
            Some("1"),
            &allowed
        ));
        assert!(!browser_request_allowed(
            Some("http://localhost:5173"),
            None,
            &allowed
        ));
    }
}
