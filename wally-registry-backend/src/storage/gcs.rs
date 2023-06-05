use std::convert::Infallible;
use std::io::Cursor;

use async_trait::async_trait;
use cloud_storage_lite::client::{BucketClient, GcsBucketClient};
use futures::TryStreamExt;
use libwally::package_id::PackageId;
use moka::sync::Cache;

use super::{StorageBackend, StorageOutput};

pub struct GcsStorage {
    client: GcsBucketClient,
    cache: Option<Cache<PackageId, Vec<u8>>>,
}

impl GcsStorage {
    pub fn new(client: GcsBucketClient, cache_size: Option<u64>) -> Self {
        if let Some(cache_size) = cache_size {
            println!("Using storage moka caching (size: {cache_size})");
        }

        Self {
            client,
            cache: cache_size.map(Cache::new),
        }
    }
}

#[async_trait]
impl StorageBackend for GcsStorage {
    async fn read(&self, key: &PackageId) -> anyhow::Result<StorageOutput> {
        if let Some(cache) = &self.cache {
            if cache.contains_key(key) {
                return Ok(Box::new(Cursor::new(cache.get(key).unwrap())));
            }
        }

        let name = key.to_string();
        let stream = self.client.download_object(&name).await?;
        let data = stream.map_ok(|chunk| chunk.to_vec()).try_concat().await?;

        if let Some(cache) = &self.cache {
            cache.insert(key.clone(), data.clone());
        }

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
