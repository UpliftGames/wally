#[cfg(feature = "postgres-analytics")]
mod postgres;

use libwally::package_id::PackageId;
use serde::{Deserialize, Serialize};

#[cfg(feature = "postgres-analytics")]
pub use postgres::PostgresAnalytics;

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum AnalyticsMode {
    #[cfg(feature = "postgres-analytics")]
    #[serde(rename_all = "kebab-case")]
    Postgres {
        database_url: String,
        downloads_table_name: String,
    },
}

#[derive(Clone)]
pub enum AnalyticsBackend {
    #[cfg(feature = "postgres-analytics")]
    Postgres(PostgresAnalytics),
}

#[async_trait]
pub trait AnalyticsBackendProvider: Send + Sync + Clone + 'static {
    async fn record_download(self, package_id: PackageId) -> anyhow::Result<()>;
    async fn ensure_initialized(&self) -> anyhow::Result<()>;
}

#[async_trait]
impl AnalyticsBackendProvider for AnalyticsBackend {
    async fn record_download(self, package_id: PackageId) -> anyhow::Result<()> {
        match self {
            #[cfg(feature = "postgres-analytics")]
            AnalyticsBackend::Postgres(backend) => backend.record_download(package_id).await,
        }
    }

    async fn ensure_initialized(&self) -> anyhow::Result<()> {
        match *self {
            #[cfg(feature = "postgres-analytics")]
            AnalyticsBackend::Postgres(backend) => backend.ensure_initialized().await,
        }
    }
}
