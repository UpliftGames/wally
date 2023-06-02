use std::convert::Infallible;
use std::io::Cursor;

use async_trait::async_trait;
use cloud_storage_lite::client::{BucketClient, GcsBucketClient};
use futures::TryStreamExt;
use libwally::package_id::PackageId;

use super::{StorageBackend, StorageOutput};

pub struct GcsStorage {
    client: GcsBucketClient,
}

impl GcsStorage {
    pub fn new(client: GcsBucketClient) -> Self {
        Self { client }
    }
}

#[async_trait]
impl StorageBackend for GcsStorage {
    async fn read(&self, key: &PackageId) -> anyhow::Result<StorageOutput> {
        let name = key.to_string();
        let stream = self.client.download_object(&name).await?;
        let data = stream.map_ok(|chunk| chunk.to_vec()).try_concat().await?;
        Ok(Box::new(Cursor::new(data)))
    }

    async fn write(&self, id: &PackageId, contents: &[u8]) -> anyhow::Result<()> {
        let contents = contents.to_vec();
        let name = id.to_string();
        self.client
            .create_object(
                &name,
                futures::stream::once(futures::future::ok::<_, Infallible>(contents)),
            )
            .await?;

        Ok(())
    }
}
