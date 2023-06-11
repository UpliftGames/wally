mod postgres;

use libwally::package_id::PackageId;
use serde::{Deserialize, Serialize};

pub use postgres::PostgresAnalytics;

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum AnalyticsMode {
    #[cfg(feature = "influx")]
    #[serde(rename_all = "kebab-case")]
    PostgresMode {},
}

#[async_trait]
pub trait AnalyticsBackend: Send + Sync {
    async fn record_download(&self, package_id: PackageId) -> anyhow::Result<()>;
    async fn ensure_initialised(&self) -> anyhow::Result<()>;
}
