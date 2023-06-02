use std::collections::BTreeSet;
use std::path::PathBuf;

use structopt::StructOpt;

use crate::installation::InstallationContext;
use crate::lockfile::{LockPackage, Lockfile};
use crate::manifest::Manifest;
use crate::package_id::PackageId;
use crate::package_source::{
    PackageSource, PackageSourceId, PackageSourceMap, Registry, TestRegistry,
};
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

        let default_registry: Box<PackageSource> = if global.test_registry {
            Box::new(PackageSource::TestRegistry(TestRegistry::new(
                &manifest.package.registry,
            )))
        } else {
            Box::new(PackageSource::Registry(Registry::from_registry_spec(
                &manifest.package.registry,
            )?))
        };

        let mut package_sources = PackageSourceMap::new(default_registry);
        package_sources.add_fallbacks()?;

        let mut try_to_use = BTreeSet::new();
        for package in lockfile.packages {
            match package {
                LockPackage::Registry(registry_package) => {
                    try_to_use.insert(PackageId::new(
                        registry_package.name,
                        registry_package.version,
                    ));
                }
                LockPackage::Git(_) => {}
            }
        }

        let resolved = resolve(&manifest, &try_to_use, &package_sources)?;

        let lockfile = Lockfile::from_resolve(&resolved);
        lockfile.save(&self.project_path)?;

        let root_package_id = PackageId::new(manifest.package.name, manifest.package.version);
        let installation = InstallationContext::new(
            &self.project_path,
            manifest.place.shared_packages,
            manifest.place.server_packages,
        );

        installation.clean()?;
        installation.install(package_sources, root_package_id, resolved)?;

        Ok(())
    }
}
