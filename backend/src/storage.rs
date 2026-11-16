//! Attachment bytes.
//!
//! Development uses a local directory. When `S3_BUCKET` is set, the same keys
//! go to S3-compatible storage. Downloads always pass through the API.

use std::sync::Arc;

use object_store::aws::AmazonS3Builder;
use object_store::local::LocalFileSystem;
use object_store::path::Path;
use object_store::{ObjectStore, ObjectStoreExt, PutPayload};

use crate::api::AppError;
use crate::config::AppConfig;

#[derive(Clone)]
pub struct BlobStore {
    inner: Arc<dyn ObjectStore>,
}

impl std::fmt::Debug for BlobStore {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("BlobStore")
    }
}

impl BlobStore {
    pub fn open(config: &AppConfig) -> anyhow::Result<Self> {
        let inner: Arc<dyn ObjectStore> = if let Some(bucket) = &config.storage.bucket {
            let access = config
                .storage
                .access_key
                .clone()
                .ok_or_else(|| anyhow::anyhow!("S3_ACCESS_KEY_ID is required"))?;
            let secret = config
                .storage
                .secret_key
                .clone()
                .ok_or_else(|| anyhow::anyhow!("S3_SECRET_ACCESS_KEY is required"))?;
            let mut builder = AmazonS3Builder::new()
                .with_bucket_name(bucket)
                .with_region(&config.storage.region)
                .with_access_key_id(access)
                .with_secret_access_key(secret);
            if let Some(endpoint) = &config.storage.endpoint {
                let http = endpoint.starts_with("http://");
                builder = builder.with_endpoint(endpoint).with_allow_http(http);
            }
            tracing::info!(bucket, "attachment storage is S3-compatible");
            Arc::new(builder.build()?)
        } else {
            std::fs::create_dir_all(&config.storage.directory)?;
            if config.env == crate::config::Environment::Production {
                tracing::warn!("attachment storage is a local directory");
            } else {
                tracing::info!("attachment storage is a local directory");
            }
            Arc::new(LocalFileSystem::new_with_prefix(&config.storage.directory)?)
        };
        Ok(Self { inner })
    }

    pub async fn put(&self, key: &str, bytes: &[u8]) -> Result<(), AppError> {
        self.inner
            .put(&Path::from(key), PutPayload::from(bytes.to_vec()))
            .await
            .map_err(store_error)?;
        Ok(())
    }

    pub async fn get(&self, key: &str) -> Result<Vec<u8>, AppError> {
        let result = self
            .inner
            .get(&Path::from(key))
            .await
            .map_err(store_error)?;
        let bytes = result.bytes().await.map_err(store_error)?;
        Ok(bytes.to_vec())
    }

    pub async fn delete(&self, key: &str) {
        if let Err(error) = self.inner.delete(&Path::from(key)).await
            && !matches!(error, object_store::Error::NotFound { .. })
        {
            tracing::warn!(%error, "attachment object was not removed");
        }
    }
}

fn store_error(error: object_store::Error) -> AppError {
    tracing::error!(%error, "attachment storage failed");
    AppError::Internal
}
