use std::time::Duration;

use sqlx::PgPool;

use crate::config::{AppConfig, Environment};
use crate::live::LiveHub;
use crate::providers::ResendClient;
use crate::storage::BlobStore;

#[derive(Clone, Debug)]
pub struct AppState {
    pub config: AppConfig,
    pub db: Option<PgPool>,
    pub provider: Option<ResendClient>,
    pub storage: BlobStore,
    pub live: LiveHub,
}

impl AppState {
    pub fn without_database(config: AppConfig) -> anyhow::Result<Self> {
        let provider = provider_from(&config);
        let storage = BlobStore::open(&config)?;
        Ok(Self {
            config,
            db: None,
            provider,
            storage,
            live: LiveHub::new(),
        })
    }

    pub async fn connect(config: AppConfig) -> anyhow::Result<Self> {
        let db = match config.database_url.as_deref() {
            Some(url) => match connect_pool(url).await {
                Ok(pool) => {
                    tracing::info!("database pool is ready");
                    Some(pool)
                }
                Err(_) => {
                    tracing::warn!("database connection failed");
                    None
                }
            },
            None => {
                tracing::warn!("DATABASE_URL is not set");
                None
            }
        };

        let provider = provider_from(&config);
        let storage = BlobStore::open(&config)?;
        Ok(Self {
            config,
            db,
            provider,
            storage,
            live: LiveHub::new(),
        })
    }

    pub fn with_pool(mut self, pool: PgPool) -> Self {
        self.db = Some(pool);
        self
    }

    pub async fn database_connected(&self) -> bool {
        let Some(pool) = &self.db else {
            return false;
        };
        sqlx::query("SELECT 1").execute(pool).await.is_ok()
    }
}

pub async fn build_state(config: AppConfig) -> anyhow::Result<AppState> {
    let state = AppState::connect(config).await?;
    if state.config.env == Environment::Production && state.db.is_none() {
        anyhow::bail!("production requires a reachable database");
    }
    Ok(state)
}

fn provider_from(config: &AppConfig) -> Option<ResendClient> {
    let key = config.resend.api_key.clone()?;
    match ResendClient::new(config.resend.api_base.clone(), key) {
        Ok(client) => Some(client),
        Err(error) => {
            tracing::error!(%error, "resend client could not be created");
            None
        }
    }
}

async fn connect_pool(url: &str) -> Result<PgPool, sqlx::Error> {
    sqlx::postgres::PgPoolOptions::new()
        .max_connections(10)
        .acquire_timeout(Duration::from_secs(5))
        .connect(url)
        .await
}
