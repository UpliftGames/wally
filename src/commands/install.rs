use std::path::PathBuf;

use structopt::StructOpt;

use crate::installation::InstallationContext;
use crate::lockfile::Lockfile;
use crate::manifest::Manifest;
use crate::package_id::PackageId;
use crate::package_source::{PackageSource, PackageSourceMap, Registry, TestRegistry};

use crate::resolution::resolve;

use super::GlobalOptions;

/// Install all of the dependencies of this project.
#[derive(Debug, StructOpt)]
pub struct InstallSubcommand {
    /// Path to the project to install dependencies for.
    #[structopt(long = "project-path", default_value = ".")]
    pub project_path: PathBuf,
}

impl InstallSubcommand {
    pub fn run(self, global: GlobalOptions) -> anyhow::Result<()> {
        let manifest = Manifest::load(&self.project_path)?;

        let lockfile = Lockfile::load(&self.project_path)?
            .unwrap_or_else(|| Lockfile::from_manifest(&manifest));

        let default_registry: Box<dyn PackageSource> = if global.test_registry {
            Box::new(TestRegistry::new(&manifest.package.registry))
        } else {
            Box::new(Registry::from_registry_spec(&manifest.package.registry)?)
        };

        let mut package_sources = PackageSourceMap::new(default_registry);
        package_sources.add_fallbacks()?;

        let try_to_use = lockfile.get_try_to_use();

        let resolved = resolve(&manifest, &self.project_path, &try_to_use, &package_sources)?;

        let lockfile = Lockfile::from_resolve(&resolved);
        lockfile.save(&self.project_path)?;

        let root_package_id = PackageId::new(manifest.package.name, manifest.package.version);
        let installation = InstallationContext::new(
            &self.project_path,
            manifest.place.shared_packages,
            manifest.place.server_packages,
        );

        installation.clean()?;
        installation.install(&package_sources, root_package_id, &resolved)?;

        Ok(())
    }
}
