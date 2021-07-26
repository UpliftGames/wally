mod gcs;
mod local;

use std::path::PathBuf;

use async_trait::async_trait;
use libwally::package_id::PackageId;
use serde::{Deserialize, Serialize};
use tokio::io::AsyncRead;

pub use gcs::GcsStorage;
pub use local::LocalStorage;

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum StorageMode {
    Local { path: Option<PathBuf> },
    Gcs { bucket: String },
}

pub type StorageOutput = Box<dyn AsyncRead + Unpin + Send + Sync + 'static>;

#[async_trait]
pub trait StorageBackend: Send + Sync + 'static {
    async fn read(&self, id: &PackageId) -> anyhow::Result<StorageOutput>;
    async fn write(&self, id: &PackageId, contents: &[u8]) -> anyhow::Result<()>;
}
