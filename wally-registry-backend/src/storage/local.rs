use std::path::{Path, PathBuf};

use anyhow::Context;
use async_trait::async_trait;
use libwally::package_id::PackageId;
use tokio::fs::{create_dir_all, File, OpenOptions};
use tokio::io::AsyncWriteExt;

use super::{StorageBackend, StorageOutput};

pub struct LocalStorage {
    path: Option<PathBuf>,
}

impl LocalStorage {
    pub fn new(path: Option<PathBuf>) -> Self {
        Self { path }
    }
}

#[async_trait]
impl StorageBackend for LocalStorage {
    async fn read(&self, id: &PackageId) -> anyhow::Result<StorageOutput> {
        let path = package_path(self.path.as_deref(), id)?;
        let file = File::open(&path)
            .await
            .with_context(|| format!("could not open path for reading {}", path.display()))?;

        Ok(Box::new(file))
    }

    async fn write(&self, id: &PackageId, contents: &[u8]) -> anyhow::Result<()> {
        let path = package_path(self.path.as_deref(), id)?;
        let directory = path.parent().unwrap();

        create_dir_all(&directory)
            .await
            .with_context(|| format!("could not create directory {}", directory.display()))?;

        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .await
            .with_context(|| format!("could not open path for writing {}", path.display()))?;

        file.write_all(contents).await?;
        Ok(())
    }
}

fn package_path(package_directory: Option<&Path>, id: &PackageId) -> anyhow::Result<PathBuf> {
    let base_path = package_directory.unwrap_or_else(|| Path::new("packages"));
    let mut path = base_path.join(id.name().scope());
    path.push(id.name().name());
    path.push(format!("{}.zip", id.version()));

    anyhow::ensure!(
        path.starts_with(base_path),
        "Cannot escape packages directory"
    );

    Ok(path)
}
