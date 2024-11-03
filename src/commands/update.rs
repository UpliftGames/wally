use std::collections::BTreeSet;

use std::path::PathBuf;
use std::str::FromStr;
use std::time::Duration;

use crate::installation::InstallationContext;
use crate::lockfile::Lockfile;
use crate::manifest::Manifest;
use crate::package_id::PackageId;
use crate::package_name::PackageName;
use crate::package_req::PackageReq;
use crate::package_source::{PackageSource, PackageSourceMap, Registry, TestRegistry};
use crate::{resolution, GlobalOptions};
use crossterm::style::{Attribute, Color, SetAttribute, SetForegroundColor};
use indicatif::{ProgressBar, ProgressStyle};
use structopt::StructOpt;

use super::utils::{generate_dependency_changes, render_update_difference};

/// Update all of the dependencies of this project.
#[derive(Debug, StructOpt)]
pub struct UpdateSubcommand {
    /// Path to the project to publish.
    #[structopt(long = "project-path", default_value = ".")]
    pub project_path: PathBuf,

    /// An optional list of dependencies to update.
    /// They must be valid package name with an optional version requirement.
    pub package_specs: Vec<PackageSpec>,
}

impl UpdateSubcommand {
    pub fn run(self, global: GlobalOptions) -> anyhow::Result<()> {
        let manifest = Manifest::load(&self.project_path)?;

        let lockfile = match Lockfile::load(&self.project_path)? {
            Some(lockfile) => lockfile,
            None => Lockfile::from_manifest(&manifest),
        };

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

        // If the user didn't specify any targets, then update all of the packages.
        // Otherwise, find the target packages to update.
        let try_to_use = if self.package_specs.is_empty() {
            println!(
                "{}   Selected {} all dependencies to try update",
                SetForegroundColor(Color::DarkGreen),
                SetForegroundColor(Color::Reset)
            );

            BTreeSet::new()
        } else {
            let try_to_use: BTreeSet<PackageId> = lockfile
                .as_ids()
                // We update the target packages by removing the package from the list of packages to try to keep.
                .filter(|package_id| !self.given_package_id_satisifies_targets(package_id))
                .collect();

            println!(
                "{}   Selected {}{} dependencies to try update",
                SetForegroundColor(Color::DarkGreen),
                SetForegroundColor(Color::Reset),
                lockfile.packages.len() - try_to_use.len(),
            );

            try_to_use
        };

        let progress = ProgressBar::new(0)
            .with_style(
                ProgressStyle::with_template("{spinner:.cyan}{wide_msg}")?.tick_chars("⠁⠈⠐⠠⠄⠂ "),
            )
            .with_message(format!(
                "{} Resolving {}new dependencies...",
                SetForegroundColor(Color::DarkGreen),
                SetForegroundColor(Color::Reset)
            ));

        let resolved_graph = resolution::resolve(&manifest, &try_to_use, &package_sources)?;

        progress.println(format!(
            "{}   Resolved {}{} total dependencies",
            SetForegroundColor(Color::DarkGreen),
            SetForegroundColor(Color::Reset),
            resolved_graph.activated.len() - 1
        ));

        progress.enable_steady_tick(Duration::from_millis(100));
        progress.suspend(|| {
            let dependency_changes = generate_dependency_changes(
                &lockfile.as_ids().collect(),
                &resolved_graph.activated,
            );
            render_update_difference(&dependency_changes, &mut std::io::stdout()).unwrap();
        });

        Lockfile::from_resolve(&resolved_graph).save(&self.project_path)?;

        progress.println(format!(
            "{}    Updated {}lockfile",
            SetForegroundColor(Color::DarkGreen),
            SetForegroundColor(Color::Reset)
        ));

        let root_package_id = manifest.package_id();
        let installation_context = InstallationContext::new(
            &self.project_path,
            manifest.place.shared_packages,
            manifest.place.server_packages,
        );

        progress.set_message(format!(
            "{}  Cleaning {}package destination...",
            SetForegroundColor(Color::DarkGreen),
            SetForegroundColor(Color::Reset)
        ));

        installation_context.clean()?;

        progress.println(format!(
            "{}    Cleaned {}package destination",
            SetForegroundColor(Color::DarkGreen),
            SetForegroundColor(Color::Reset)
        ));

        progress.finish_with_message(format!(
            "{}{}  Starting installation {}",
            SetAttribute(Attribute::Bold),
            SetForegroundColor(Color::DarkGreen),
            SetForegroundColor(Color::Reset)
        ));

        installation_context.install(package_sources, root_package_id, resolved_graph)?;

        Ok(())
    }

    fn given_package_id_satisifies_targets(&self, package_id: &PackageId) -> bool {
        self.package_specs
            .iter()
            .any(|target_package| match target_package {
                PackageSpec::Named(named_target) => package_id.name() == named_target,
                PackageSpec::Required(required_target) => {
                    required_target.matches(package_id.name(), package_id.version())
                }
            })
    }
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub enum PackageSpec {
    Named(PackageName),
    Required(PackageReq),
}

impl FromStr for PackageSpec {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> anyhow::Result<Self> {
        if let Ok(package_req) = value.parse() {
            Ok(PackageSpec::Required(package_req))
        } else if let Ok(package_name) = value.parse() {
            Ok(PackageSpec::Named(package_name))
        } else {
            anyhow::bail!(
                "Was unable to parse {} into a package requirement or a package name!",
                value
            )
        }
    }
}
