use std::collections::BTreeSet;
use std::path::PathBuf;
use std::time::Duration;

use anyhow::bail;
use crossterm::style::{Color, SetForegroundColor};
use indicatif::{ProgressBar, ProgressStyle};
use indoc::indoc;
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

    /// Flag to error if the lockfile does not match with the latest dependencies.
    #[structopt(long = "latest-lock")]
    pub latest_lock: bool,
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

        if self.latest_lock && resolved.activated != try_to_use {
            // We know at this point that these are not equal sets.
            // Meaning, at least there was a new dependency being used or an old dependency that is no longer being used.
            // We'll find either by taking the difference of the latest set of dependencies and the old set of dependencies.
            let old_dependencies: Vec<_> = try_to_use.difference(&resolved.activated).collect();
            let latest_dependencies: Vec<_> = resolved.activated.difference(&try_to_use).collect();

            // If a dependency name is present in both sets, then it was updated.
            let updated_dependencies = {
                let mut updated_ids = Vec::new();

                for old_id in old_dependencies.iter() {
                    for new_id in latest_dependencies.iter() {
                        if old_id.name() == new_id.name() {
                            updated_ids.push((old_id, new_id));
                            break;
                        }
                    }
                }

                updated_ids
            };

            // If there is a dependency in the latest set, but not in the old set, then it is a new dependency.
            let gained_dependencies: Vec<_> = latest_dependencies
                .iter()
                .filter(|new_id| {
                    !old_dependencies
                        .iter()
                        .any(|old_id| old_id.name() == new_id.name())
                })
                .collect();

            // If there is a dependency in the old set, but not in the latest, then it's a dependency no longer used.
            let lost_dependencies: Vec<_> = old_dependencies
                .iter()
                .filter(|old_id| {
                    !latest_dependencies
                        .iter()
                        .any(|new_id| new_id.name() == old_id.name())
                })
                .collect();

            let mut formatted_result: String = updated_dependencies
                .iter()
                .map(|(old, new)| format!("Updated {} to {}\n", old.to_string(), new.to_string()))
                .chain(
                    gained_dependencies
                        .iter()
                        .map(|new| format!("Added {}\n", new)),
                )
                .chain(
                    lost_dependencies
                        .iter()
                        .map(|old| format!("Removed {}\n", old)),
                )
                .collect();

            // There'll be an extra new-line at the end, we can pop it off for comestic reasons.
            formatted_result.pop();

            bail!(
                indoc! {r#"
                The dependencies and their versions being installed do not match with the lockfile! These are the conflicts:
    
                {}
            "#},
                formatted_result
            )
        }

        let new_lockfile = Lockfile::from_resolve(&resolved);
        new_lockfile.save(&self.project_path)?;

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
