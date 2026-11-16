use std::fmt;
use std::net::SocketAddr;
use std::path::PathBuf;

use axum::http::HeaderValue;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppConfig {
    pub env: Environment,
    pub host: String,
    pub port: u16,
    pub database_url: Option<String>,
    pub cors_origins: Vec<String>,
    pub log_format: LogFormat,
    pub resend: ResendConfig,
    pub storage: StorageConfig,
    pub scanner: Option<String>,
}

#[derive(Clone, PartialEq, Eq)]
pub struct StorageConfig {
    pub directory: PathBuf,
    pub bucket: Option<String>,
    pub region: String,
    pub endpoint: Option<String>,
    pub access_key: Option<String>,
    pub secret_key: Option<String>,
}

impl fmt::Debug for StorageConfig {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("StorageConfig")
            .field("directory", &self.directory)
            .field("bucket", &self.bucket)
            .field("region", &self.region)
            .field("endpoint", &self.endpoint)
            .field("access_key", &self.access_key.as_ref().map(|_| "set"))
            .field("secret_key", &self.secret_key.as_ref().map(|_| "set"))
            .finish()
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct ResendConfig {
    pub api_key: Option<String>,
    pub webhook_secret: Option<String>,
    pub api_base: String,
    pub sending_domain: String,
}

impl fmt::Debug for ResendConfig {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ResendConfig")
            .field("api_key", &self.api_key.as_ref().map(|_| "set"))
            .field(
                "webhook_secret",
                &self.webhook_secret.as_ref().map(|_| "set"),
            )
            .field("api_base", &self.api_base)
            .field("sending_domain", &self.sending_domain)
            .finish()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Environment {
    Development,
    Test,
    Production,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogFormat {
    Pretty,
    Json,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ConfigError {
    #[error("missing required environment variable {0}")]
    Missing(&'static str),
    #[error("invalid value for {0}")]
    Invalid(&'static str),
}

impl AppConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        Self::from_lookup(|key| std::env::var(key).ok())
    }

    pub fn from_lookup(
        mut lookup: impl FnMut(&str) -> Option<String>,
    ) -> Result<Self, ConfigError> {
        let env = Environment::parse(
            lookup("APP_ENV")
                .unwrap_or_else(|| "development".to_string())
                .trim(),
        )?;
        let host = lookup("APP_HOST").unwrap_or_else(|| "127.0.0.1".to_string());
        let host = host.trim().to_string();
        if host.is_empty() {
            return Err(ConfigError::Invalid("APP_HOST"));
        }
        let port = parse_port(
            lookup("APP_PORT")
                .unwrap_or_else(|| "8080".to_string())
                .trim(),
        )?;
        let database_url = lookup("DATABASE_URL")
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());
        let cors_origins = parse_origins(lookup("CORS_ORIGINS").as_deref())?;
        let log_format = LogFormat::parse(
            lookup("LOG_FORMAT")
                .unwrap_or_else(|| "pretty".to_string())
                .trim(),
        )?;
        let resend = ResendConfig {
            api_key: blank(lookup("RESEND_API_KEY")),
            webhook_secret: blank(lookup("RESEND_WEBHOOK_SECRET")),
            api_base: lookup("RESEND_API_BASE")
                .map(|value| value.trim().trim_end_matches('/').to_string())
                .filter(|value| !value.is_empty())
                .unwrap_or_else(|| "https://api.resend.com".to_string()),
            sending_domain: normalize_domain(
                lookup("RESEND_SENDING_DOMAIN")
                    .unwrap_or_else(|| "kamfany.com".to_string())
                    .trim(),
            )?,
        };
        if !(resend.api_base.starts_with("https://") || resend.api_base.starts_with("http://")) {
            return Err(ConfigError::Invalid("RESEND_API_BASE"));
        }
        let storage = storage_from(&mut lookup)?;
        let scanner = blank(lookup("CLAMAV_ADDRESS"));
        if let Some(address) = &scanner {
            address
                .parse::<SocketAddr>()
                .map_err(|_| ConfigError::Invalid("CLAMAV_ADDRESS"))?;
        }

        if env == Environment::Production && database_url.is_none() {
            return Err(ConfigError::Missing("DATABASE_URL"));
        }

        Ok(Self {
            env,
            host,
            port,
            database_url,
            cors_origins,
            log_format,
            resend,
            storage,
            scanner,
        })
    }

    pub fn socket_addr(&self) -> Result<SocketAddr, ConfigError> {
        format!("{}:{}", self.host, self.port)
            .parse()
            .map_err(|_| ConfigError::Invalid("APP_HOST"))
    }
}

impl Environment {
    fn parse(value: &str) -> Result<Self, ConfigError> {
        match value.to_ascii_lowercase().as_str() {
            "development" | "dev" => Ok(Self::Development),
            "test" => Ok(Self::Test),
            "production" | "prod" => Ok(Self::Production),
            _ => Err(ConfigError::Invalid("APP_ENV")),
        }
    }
}

impl LogFormat {
    fn parse(value: &str) -> Result<Self, ConfigError> {
        match value.to_ascii_lowercase().as_str() {
            "pretty" => Ok(Self::Pretty),
            "json" => Ok(Self::Json),
            _ => Err(ConfigError::Invalid("LOG_FORMAT")),
        }
    }
}

fn storage_from(
    lookup: &mut impl FnMut(&str) -> Option<String>,
) -> Result<StorageConfig, ConfigError> {
    let bucket = blank(lookup("S3_BUCKET"));
    let access_key = blank(lookup("S3_ACCESS_KEY_ID"));
    let secret_key = blank(lookup("S3_SECRET_ACCESS_KEY"));
    if bucket.is_some() && (access_key.is_none() || secret_key.is_none()) {
        return Err(ConfigError::Missing("S3_ACCESS_KEY_ID"));
    }
    let directory = lookup("STORAGE_DIR")
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "storage".to_string());
    let region = lookup("S3_REGION")
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "auto".to_string());
    Ok(StorageConfig {
        directory: PathBuf::from(directory),
        bucket,
        region,
        endpoint: blank(lookup("S3_ENDPOINT")),
        access_key,
        secret_key,
    })
}

fn blank(value: Option<String>) -> Option<String> {
    value
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
}

fn normalize_domain(value: &str) -> Result<String, ConfigError> {
    let domain = value.trim().to_ascii_lowercase();
    if domain.is_empty()
        || !domain.contains('.')
        || domain.contains([' ', '/', '@'])
        || domain.starts_with('.')
        || domain.ends_with('.')
    {
        return Err(ConfigError::Invalid("RESEND_SENDING_DOMAIN"));
    }
    Ok(domain)
}

fn parse_port(value: &str) -> Result<u16, ConfigError> {
    value.parse().map_err(|_| ConfigError::Invalid("APP_PORT"))
}

fn parse_origins(value: Option<&str>) -> Result<Vec<String>, ConfigError> {
    let raw = value.unwrap_or("http://localhost:5173,http://127.0.0.1:5173");
    let origins: Vec<String> = raw
        .split(',')
        .map(str::trim)
        .filter(|origin| !origin.is_empty())
        .map(str::to_string)
        .collect();
    if origins.is_empty() {
        return Err(ConfigError::Invalid("CORS_ORIGINS"));
    }
    for origin in &origins {
        HeaderValue::from_str(origin).map_err(|_| ConfigError::Invalid("CORS_ORIGINS"))?;
    }
    Ok(origins)
}

#[cfg(test)]
mod tests {
    use super::{AppConfig, ConfigError, Environment, LogFormat};

    #[test]
    fn development_defaults_do_not_require_a_database() {
        let config = AppConfig::from_lookup(|_| None).expect("defaults");
        assert_eq!(config.env, Environment::Development);
        assert_eq!(config.host, "127.0.0.1");
        assert_eq!(config.port, 8080);
        assert_eq!(config.database_url, None);
        assert_eq!(config.log_format, LogFormat::Pretty);
        assert_eq!(
            config.cors_origins,
            vec![
                "http://localhost:5173".to_string(),
                "http://127.0.0.1:5173".to_string()
            ]
        );
        assert_eq!(
            config.socket_addr().expect("address").to_string(),
            "127.0.0.1:8080"
        );
    }

    #[test]
    fn production_requires_a_database_url() {
        let error = AppConfig::from_lookup(|key| match key {
            "APP_ENV" => Some("production".to_string()),
            _ => None,
        })
        .expect_err("production without a database");
        assert_eq!(error, ConfigError::Missing("DATABASE_URL"));
    }

    #[test]
    fn blank_database_url_is_treated_as_unset() {
        let error = AppConfig::from_lookup(|key| match key {
            "APP_ENV" => Some("prod".to_string()),
            "DATABASE_URL" => Some("   ".to_string()),
            _ => None,
        })
        .expect_err("blank database url");
        assert_eq!(error, ConfigError::Missing("DATABASE_URL"));
    }

    #[test]
    fn production_accepts_a_present_database_url() {
        let config = AppConfig::from_lookup(|key| match key {
            "APP_ENV" => Some("production".to_string()),
            "DATABASE_URL" => Some("postgres://mail@db/kamfany".to_string()),
            "LOG_FORMAT" => Some("json".to_string()),
            _ => None,
        })
        .expect("config");
        assert_eq!(config.env, Environment::Production);
        assert_eq!(
            config.database_url.as_deref(),
            Some("postgres://mail@db/kamfany")
        );
        assert_eq!(config.log_format, LogFormat::Json);
    }

    #[test]
    fn debug_output_does_not_include_the_resend_key() {
        let config = AppConfig::from_lookup(|key| match key {
            "RESEND_API_KEY" => Some("re_example_not_a_real_key".to_string()),
            "RESEND_WEBHOOK_SECRET" => Some("whsec_example".to_string()),
            _ => None,
        })
        .expect("config");
        let rendered = format!("{config:?}");
        assert!(!rendered.contains("re_example_not_a_real_key"));
        assert!(!rendered.contains("whsec_example"));
        assert!(rendered.contains("kamfany.com"));
    }

    #[test]
    fn invalid_port_is_rejected() {
        let error = AppConfig::from_lookup(|key| match key {
            "APP_PORT" => Some("mail".to_string()),
            _ => None,
        })
        .expect_err("bad port");
        assert_eq!(error, ConfigError::Invalid("APP_PORT"));
    }

    #[test]
    fn clamav_address_must_be_a_socket() {
        let error = AppConfig::from_lookup(|key| match key {
            "CLAMAV_ADDRESS" => Some("clamav".to_string()),
            _ => None,
        })
        .expect_err("bad scanner");
        assert_eq!(error, ConfigError::Invalid("CLAMAV_ADDRESS"));
    }
}
