use super::AnalyticsBackend;
use chrono::{DateTime, Utc};
use sqlx::{Pool, Postgres};
#[cfg(feature = "influx")]

pub struct PostgresAnalytics {
    pool: Pool<Postgres>,
}

impl PostgresAnalytics {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl AnalyticsBackend for PostgresAnalytics {
    async fn record_download(
        &self,
        package_id: libwally::package_id::PackageId,
    ) -> anyhow::Result<()> {
        println!("{}", Utc::now().format("%+").to_string());

        sqlx::query("INSERT INTO test_table VALUES (NOW(), $2, $3, $4);")
            .bind(Utc::now())
            .bind(package_id.name().scope())
            .bind(package_id.name().name())
            .bind(package_id.version().to_string())
            .fetch_all(&self.pool)
            .await?;

        Ok(())
    }

    async fn ensure_initialised(&self) -> anyhow::Result<()> {
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS test_table (
            time TIMESTAMP NOT NULL,
            package_scope VARCHAR ( 50 ) NOT NULL,
            package_name VARCHAR ( 50 ) NOT NULL,
            package_version VARCHAR ( 50 ) NOT NULL
         );",
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}

struct DownloadRecord {
    time: DateTime<Utc>,
    package_scope: String,
    package_name: String,
    package_version: String,
}
