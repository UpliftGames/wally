use std::path::PathBuf;
use structopt::StructOpt;

use crate::{auth::AuthStore, manifest::Manifest, package_index::PackageIndex};

/// Log out of a registry.
#[derive(Debug, StructOpt)]
pub struct LogoutSubcommand {
    /// Path to a project to decide how to logout
    #[structopt(long = "project-path", default_value = ".")]
    pub project_path: PathBuf,
}

impl LogoutSubcommand {
    pub fn run(self) -> anyhow::Result<()> {
        let manifest = Manifest::load(&self.project_path)?;
        let registry = url::Url::parse(&manifest.package.registry)?;
        let package_index = PackageIndex::new(&registry, None)?;
        let api = package_index.config()?.api;

        AuthStore::set_token(api.as_str(), None)?;

        Ok(())
    }
}
