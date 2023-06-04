use std::collections::BTreeSet;
use std::path::PathBuf;
use std::time::Duration;

use crossterm::style::{Color, SetForegroundColor};
use indicatif::{ProgressBar, ProgressStyle};
use structopt::StructOpt;

use crate::installation::InstallationContext;
use crate::lockfile::{LockPackage, Lockfile};
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

        let progress = ProgressBar::new(0)
            .with_style(
                ProgressStyle::with_template("{spinner:.cyan}{wide_msg}")?.tick_chars("⠁⠈⠐⠠⠄⠂ "),
            )
            .with_message(format!(
                "{} Resolving {}packages...",
                SetForegroundColor(Color::DarkGreen),
                SetForegroundColor(Color::Reset)
            ));
        progress.enable_steady_tick(Duration::from_millis(100));

        let resolved = resolve(&manifest, &try_to_use, &package_sources)?;

        progress.println(format!(
            "{}   Resolved {}{} dependencies",
            SetForegroundColor(Color::DarkGreen),
            SetForegroundColor(Color::Reset),
            resolved.activated.len() - 1
        ));

        let lockfile = Lockfile::from_resolve(&resolved);
        lockfile.save(&self.project_path)?;

        progress.println(format!(
            "{}  Generated {}lockfile",
            SetForegroundColor(Color::DarkGreen),
            SetForegroundColor(Color::Reset)
        ));

        progress.set_message(format!(
            "{}  Cleaning {}package destination...",
            SetForegroundColor(Color::DarkGreen),
            SetForegroundColor(Color::Reset)
        ));
        let root_package_id = PackageId::new(manifest.package.name, manifest.package.version);
        let installation = InstallationContext::new(
            &self.project_path,
            manifest.place.shared_packages,
            manifest.place.server_packages,
        );

        installation.clean()?;
        progress.println(format!(
            "{}    Cleaned {}package destination",
            SetForegroundColor(Color::DarkGreen),
            SetForegroundColor(Color::Reset)
        ));
        progress.finish_and_clear();

        installation.install(package_sources, root_package_id, resolved)?;

        Ok(())
    }
}
