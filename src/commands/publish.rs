use std::path::PathBuf;

use anyhow::Context;
use structopt::StructOpt;

use crate::{
    auth::AuthStore, manifest::Manifest, package_contents::PackageContents,
    package_index::PackageIndex, GlobalOptions,
};

/// Publish this project to a registry.
#[derive(Debug, StructOpt)]
pub struct PublishSubcommand {
    /// Path to the project to publish.
    #[structopt(long = "project-path", default_value = ".")]
    pub project_path: PathBuf,
}

impl PublishSubcommand {
    pub fn run(self, global: GlobalOptions) -> anyhow::Result<()> {
        let manifest = Manifest::load(&self.project_path)?;
        let registry = url::Url::parse(&manifest.package.registry)?;
        let auth_store = AuthStore::load()?;
        let package_index = match global.use_temp_index {
            true => PackageIndex::new_temp(&registry, None)?,
            false => PackageIndex::new(&registry, None)?,
        };
        let api = package_index.config()?.api;
        let contents = PackageContents::pack_from_path(&self.project_path)?;

        let auth = auth_store
            .token
            .with_context(|| "Auth token is required to publish, use `wally login`")?;

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
