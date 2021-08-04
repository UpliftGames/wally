use std::path::PathBuf;

use anyhow::Context;
use structopt::StructOpt;

use crate::{
    auth::AuthStore, manifest::Manifest, package_contents::PackageContents,
    package_index::PackageIndex,
};

/// Publish this project to a registry.
#[derive(Debug, StructOpt)]
pub struct PublishSubcommand {
    /// Path to the project to publish.
    #[structopt(long = "project-path", default_value = ".")]
    pub project_path: PathBuf,
}

impl PublishSubcommand {
    pub fn run(self) -> anyhow::Result<()> {
        let manifest = Manifest::load(&self.project_path)?;
        let registry = url::Url::parse(&manifest.package.registry)?;
        let auth_store = AuthStore::load()?;
        let package_index = PackageIndex::new(&registry, None)?;
        let api = package_index.config()?.api;
        let contents = PackageContents::pack_from_path(&self.project_path)?;

        let auth = auth_store
            .tokens
            .get(api.as_str())
            .with_context(|| "Authentication is required to publish, use `wally login`")?;

        println!(
            "Publishing {} to {}",
            manifest.package_id(),
            package_index.url()
        );

        let client = reqwest::blocking::Client::new();
        let response = client
            .post(api.join("/v1/publish")?)
            .bearer_auth(auth)
            .body(contents.data().to_owned())
            .send()?;

        if response.status().is_success() {
            println!("Package published successfully!");
        } else {
            println!("Error: {}", response.status());
            println!("{}", response.text()?);
        }

        Ok(())
    }
}
