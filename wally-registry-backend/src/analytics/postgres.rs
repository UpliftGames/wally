use super::AnalyticsBackend;
use sqlx::{Pool, Postgres};
pub struct PostgresAnalytics {
    pool: Pool<Postgres>,
    table_name: String,
}

impl PostgresAnalytics {
    pub fn new(pool: Pool<Postgres>, table_name: String) -> Self {
        Self { pool, table_name }
    }
}

#[async_trait]
impl AnalyticsBackend for PostgresAnalytics {
    async fn record_download(
        &self,
        package_id: libwally::package_id::PackageId,
    ) -> anyhow::Result<()> {
        sqlx::query(&format!(
            "INSERT INTO {} VALUES (NOW(), $1, $2, $3);",
            self.table_name
        ))
        .bind(package_id.name().scope())
        .bind(package_id.name().name())
        .bind(package_id.version().to_string())
        .fetch_all(&self.pool)
        .await?;

        Ok(())
    }

    async fn ensure_initialized(&self) -> anyhow::Result<()> {
        sqlx::query(&format!(
            "CREATE TABLE IF NOT EXISTS {} (
            time TIMESTAMP NOT NULL,
            package_scope VARCHAR ( 50 ) NOT NULL,
            package_name VARCHAR ( 50 ) NOT NULL,
            package_version VARCHAR ( 50 ) NOT NULL
         );",
            self.table_name
        ))
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}
