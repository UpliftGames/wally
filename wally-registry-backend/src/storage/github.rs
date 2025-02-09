use std::io::Cursor;

use async_trait::async_trait;
use base64::{engine::general_purpose::STANDARD, Engine as _};
use libwally::package_id::PackageId;
use moka::sync::Cache;
use octorust::{types::Tagger, Client};

use super::{StorageBackend, StorageOutput};

pub struct GithubStorage {
    owner: String,
    repo: String,
    client: Client,
    cache: Option<Cache<PackageId, Vec<u8>>>,
}

impl GithubStorage {
    pub fn new(client: Client, owner: String, repo: String, cache_size: Option<u64>) -> Self {
        if let Some(cache_size) = cache_size {
            println!("Using storage moka caching (size: {cache_size})");
        }

        Self {
            owner,
            repo,
            client,
            cache: cache_size.map(Cache::new),
        }
    }
}

#[async_trait]
impl StorageBackend for GithubStorage {
    async fn read(&self, key: &PackageId) -> anyhow::Result<StorageOutput> {
        if let Some(cache) = &self.cache {
            if cache.contains_key(key) {
                return Ok(Box::new(Cursor::new(cache.get(key).unwrap())));
            }
        }

        let path = format!("{}-{}.zip", key.name(), key.version());
        let response = self
            .client
            .repos()
            .get_content(&self.owner, &self.repo, &path, "")
            .await?;

        let encoded_content = response
            .body
            .content_file()
            .ok_or(anyhow::anyhow!("File not found"))?;

        let content_str = STANDARD.decode(&encoded_content.content)?.to_vec();

        if let Some(cache) = &self.cache {
            cache.insert(key.clone(), content_str.clone());
        }

        Ok(Box::new(Cursor::new(content_str)))
    }

    async fn write(&self, id: &PackageId, contents: &[u8]) -> anyhow::Result<()> {
        let contents = contents.to_vec();
        let path = format!("{}-{}.zip", id.name(), id.version());

        self.client
            .repos()
            .create_or_update_file_contents(
                &self.owner,
                &self.repo,
                &path,
                &octorust::types::ReposCreateUpdateFileContentsRequest {
                    author: None,
                    branch: "".to_string(),
                    message: format!("Add package {}", id.to_string()),
                    content: STANDARD.encode(&contents),
                    sha: "".to_string(),
                    committer: Some(Tagger {
                        date: "".to_string(),
                        name: "PackageUser".to_string(),
                        email: "PackageUser@localhost".to_string(),
                    }),
                },
            )
            .await?;

        Ok(())
    }
}
