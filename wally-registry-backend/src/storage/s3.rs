use std::io::Cursor;

use async_trait::async_trait;
use futures::TryStreamExt;
use libwally::package_id::PackageId;
use tokio::sync::Mutex;

use rusoto_s3::{GetObjectRequest, PutObjectRequest, S3Client, S3};

use super::{StorageBackend, StorageOutput};

pub struct S3Storage {
    client: Mutex<S3Client>,
    bucket: String,
}

impl S3Storage {
    pub fn new(client: S3Client, bucket: String) -> Self {
        Self {
            client: Mutex::new(client),
            bucket,
        }
    }
}

#[async_trait]
impl StorageBackend for S3Storage {
    async fn read(&self, key: &PackageId) -> anyhow::Result<StorageOutput> {
        let name = key.to_string();
        let client = self.client.lock().await;

        let result = client
            .get_object(GetObjectRequest {
                bucket: self.bucket.to_owned(),
                key: name.to_owned(),
                ..Default::default()
            })
            .await?;

        let stream = result.body.unwrap();
        let data = stream.map_ok(|chunk| chunk.to_vec()).try_concat().await?;
        Ok(Box::new(Cursor::new(data)))
    }

    async fn write(&self, id: &PackageId, contents: &[u8]) -> anyhow::Result<()> {
        let name = id.to_string();
        let client = self.client.lock().await;
        let contents = contents.to_vec();

        client
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
