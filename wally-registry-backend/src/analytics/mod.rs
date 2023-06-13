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

#[async_trait]
pub trait AnalyticsBackend: Send + Sync {
    async fn record_download(&self, package_id: PackageId) -> anyhow::Result<()>;
    async fn ensure_initialized(&self) -> anyhow::Result<()>;
}
