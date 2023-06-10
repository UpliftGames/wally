use std::io::Cursor;

use async_trait::async_trait;
use futures::TryStreamExt;
use libwally::package_id::PackageId;
use moka::sync::Cache;

use rusoto_s3::{GetObjectRequest, PutObjectRequest, S3Client, S3};

use super::{StorageBackend, StorageOutput};

pub struct S3Storage {
    client: S3Client,
    bucket: String,
    cache: Option<Cache<PackageId, Vec<u8>>>,
}

impl S3Storage {
    pub fn new(client: S3Client, bucket: String, cache_size: Option<u64>) -> Self {
        if let Some(cache_size) = cache_size {
            println!("Using storage moka caching (size: {cache_size})");
        }

        Self {
            client,
            bucket,
            cache: cache_size.map(Cache::new),
        }
    }
}

#[async_trait]
impl StorageBackend for S3Storage {
    async fn read(&self, key: &PackageId) -> anyhow::Result<StorageOutput> {
        if let Some(cache) = &self.cache {
            if cache.contains_key(key) {
                return Ok(Box::new(Cursor::new(cache.get(key).unwrap())));
            }
        }

        let name = key.to_string();

        let result = self
            .client
            .get_object(GetObjectRequest {
                bucket: self.bucket.to_owned(),
                key: name.to_owned(),
                ..Default::default()
            })
            .await?;

        let stream = result.body.unwrap();
        let data = stream.map_ok(|chunk| chunk.to_vec()).try_concat().await?;

        if let Some(cache) = &self.cache {
            cache.insert(key.clone(), data.clone());
        }

        Ok(Box::new(Cursor::new(data)))
    }

    async fn write(&self, id: &PackageId, contents: &[u8]) -> anyhow::Result<()> {
        let name = id.to_string();
        let contents = contents.to_vec();

        self.client
            .put_object(PutObjectRequest {
                bucket: self.bucket.to_owned(),
                key: name.to_owned(),
                body: Some(contents.into()),
                ..Default::default()
            })
            .await?;

        Ok(())
    }
}
