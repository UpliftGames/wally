mod gcs;
mod local;

#[cfg(feature = "s3-storage")]
mod s3;

use std::path::PathBuf;

use async_trait::async_trait;
use libwally::package_id::PackageId;
use serde::{Deserialize, Serialize};
use tokio::io::AsyncRead;

pub use gcs::GcsStorage;
pub use local::LocalStorage;

#[cfg(feature = "s3-storage")]
pub use s3::S3Storage;

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum StorageMode {
    Local {
        path: Option<PathBuf>,
    },
    #[serde(rename_all = "kebab-case")]
    Gcs {
        bucket: String,
        // Moka cache to keep the most popular packages in memory and accelerate response times
        cache_size: Option<u64>,
    },
    #[cfg(feature = "s3-storage")]
    #[cfg(feature = "s3-storage")]
    #[serde(rename_all = "kebab-case")]
    S3 {
        bucket: String,
        cache_size: Option<u64>,
    },
}

pub type StorageOutput = Box<dyn AsyncRead + Unpin + Send + Sync + 'static>;

#[async_trait]
pub trait StorageBackend: Send + Sync + 'static {
    async fn read(&self, id: &PackageId) -> anyhow::Result<StorageOutput>;
    async fn write(&self, id: &PackageId, contents: &[u8]) -> anyhow::Result<()>;
}
