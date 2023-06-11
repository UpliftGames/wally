use std::path::{Path, PathBuf};

use anyhow::{bail, Context};
use structopt::StructOpt;
use ubyte::ToByteUnit;
use url::Url;

use crate::{
    auth::AuthStore, manifest::Manifest, package_contents::PackageContents,
    package_index::PackageIndex, GlobalOptions,
};

const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Publish this project to a registry.
#[derive(Debug, StructOpt)]
pub struct PublishSubcommand {
    /// Path to the project to publish.
    #[structopt(long = "project-path", default_value = ".")]
    pub project_path: PathBuf,

    /// Auth token to use
    #[structopt(long = "token")]
    pub token: Option<String>,
}

impl PublishSubcommand {
    pub fn run(self, global: GlobalOptions) -> anyhow::Result<()> {
        let manifest = Manifest::load(&self.project_path)?;

        if manifest.package.private {
            bail!("Cannot publish private package.");
        }

        let index_url = if global.test_registry {
            let index_path = Path::new(&manifest.package.registry)
                .join("index")
                .canonicalize()?;

            Url::from_directory_path(index_path).unwrap()
        } else {
            Url::parse(&manifest.package.registry)?
        };

        let package_index = if global.use_temp_index {
            PackageIndex::new_temp(&index_url, None)?
        } else {
            PackageIndex::new(&index_url, None)?
        };

        let api = package_index.config()?.api;
        let contents = PackageContents::pack_from_path(&self.project_path)?;

        if contents.data().len() > 2.mebibytes() {
            bail!("Package size exceeds 2MB. Reduce package size and try again.");
        }

        let auth = match self.token {
            Some(token) => token,
            None => AuthStore::get_token(api.as_str())?
                .with_context(|| "Authentication is required to publish, use `wally login`")?,
        };

        println!(
            "Publishing {} to {}",
            manifest.package_id(),
            package_index.url()
        );

        // Used by integration tests to ensure token handling is correct
        if let Some(token) = global.check_token {
            assert!(token.eq(&auth));
            return Ok(());
        }

        let client = reqwest::blocking::Client::new();
        let response = client
            .post(api.join("/v1/publish")?)
            .header("accept", "application/json")
            .header("Wally-Version", VERSION)
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
