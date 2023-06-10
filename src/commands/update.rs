use std::collections::BTreeSet;
use std::path::PathBuf;
use std::str::FromStr;

use crate::lockfile::{LockPackage, Lockfile};
use crate::manifest::Manifest;
use crate::package_id::PackageId;
use crate::package_name::PackageName;
use crate::package_req::PackageReq;
use crate::package_source::{PackageSource, PackageSourceMap, Registry, TestRegistry};
use crate::{resolution, GlobalOptions};
use structopt::StructOpt;

/// Update all of the dependencies of this project.
#[derive(Debug, StructOpt)]
pub struct UpdateSubcommand {
    /// An optional list of dependencies to update.
    /// They must be valid package name with an optional version requirement.
    pub target_packages: Vec<TargetPackage>,

    /// Path to the project to publish.
    #[structopt(long = "project-path", default_value = ".")]
    pub project_path: PathBuf,
}

impl UpdateSubcommand {
    pub fn run(self, global: GlobalOptions) -> anyhow::Result<()> {
        let manifest = Manifest::load(&self.project_path)?;
        
        let lockfile = match Lockfile::load(&self.project_path)? {
            Some(lockfile) => lockfile,
            None => {
                anyhow::bail!("Missing lockfile!")
            }
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
        let try_to_use = if self.target_packages.is_empty() {
            BTreeSet::new()
        } else {
            lockfile
                .packages
                .iter()
                .map(|lock_package| match lock_package {
                    LockPackage::Registry(lock_package) => {
                        PackageId::new(lock_package.name.clone(), lock_package.version.clone())
                    }
                    LockPackage::Git(_) => todo!(),
                })
                // We update the target packages by removing the package from the list of packages to try to keep.
                .filter(|package_id| {
                    !self.target_packages
                        .iter()
                        .any(|target_package| match target_package {
                            TargetPackage::Named(named_target) => package_id.name() == named_target,
                            TargetPackage::Required(required_target) => {
                                required_target.matches(package_id.name(), package_id.version())
                            }
                        })
                })
                .collect()
        };

        let resolved_graph = resolution::resolve(&manifest, &try_to_use, &package_sources)?;

        Lockfile::from_resolve(&resolved_graph).save(&self.project_path)?;
        let dependency_changes = generate_depedency_changes(
            &lockfile
                .packages
                .iter()
                .map(|lock_package| match lock_package {
                    LockPackage::Registry(lock_package) => {
                        PackageId::new(lock_package.name.clone(), lock_package.version.clone())
                    }
                    LockPackage::Git(_) => todo!(),
                })
                .collect(),
            &resolved_graph.activated,
        );

        render_update_difference(&dependency_changes);
        Ok(())
    }
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub enum TargetPackage {
    Named(PackageName),
    Required(PackageReq),
}

impl FromStr for TargetPackage {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> anyhow::Result<Self> {
        if let Ok(package_req) = value.parse() {
            Ok(TargetPackage::Required(package_req))
        } else if let Ok(package_name) = value.parse() {
            Ok(TargetPackage::Named(package_name))
        } else {
            // TODO: Give better error message here, I guess.
            anyhow::bail!("Was unable to parse into a package requirement or a package named.")
        }
    }
}

enum DependencyChange {
    Added(PackageId),
    Removed(PackageId),
    Updated { from: PackageId, to: PackageId },
}

fn generate_depedency_changes(
    old_dependencies: &BTreeSet<PackageId>,
    new_dependencies: &BTreeSet<PackageId>,
) -> Vec<DependencyChange> {
    let added_packages = new_dependencies.difference(&old_dependencies);
    let removed_packages = old_dependencies.difference(&new_dependencies);
    let changed_dependencies: BTreeSet<&PackageName> = added_packages
        .clone()
        .chain(removed_packages.clone())
        .map(|package| package.name())
        .collect();

    let mut depedency_changes = Vec::new();

    for dependency_name in changed_dependencies {
        let matching_packages_removed = removed_packages
            .clone()
            .filter(|x| *x.name() == *dependency_name);
        let matching_packages_added = added_packages
            .clone()
            .filter(|x| *x.name() == *dependency_name);

        match (
            matching_packages_added.clone().count(),
            matching_packages_removed.clone().count(),
        ) {
            (1, 1) => {
                // We know for certain that there is only one item in the iterator.
                let package_added = matching_packages_added.last().unwrap();
                let package_removed = matching_packages_removed.last().unwrap();
                depedency_changes.push(DependencyChange::Updated {
                    from: package_added.clone(),
                    to: package_removed.clone(),
                });
            }
            (0, 1) => {
                // We know for certain that there is only one item in the iterator.
                let package_removed = matching_packages_removed.last().unwrap();
                depedency_changes.push(DependencyChange::Removed(package_removed.clone()));
            }
            (1, 0) => {
                // We know for certain that there is only one item in the iterator.
                let package_added = matching_packages_added.last().unwrap();
                depedency_changes.push(DependencyChange::Added(package_added.clone()));
            }
            (0, 0) => panic!("Impossible for the package name {} to not be removed or added if found in earlier.", dependency_name),
            (_, _) => {
                let mut found_changes = matching_packages_added
                    .map(|package| DependencyChange::Added(package.clone()))
                    .chain(
                        matching_packages_removed
                            .map(|package| DependencyChange::Removed(package.clone())),
                    )
                    .collect();
                depedency_changes.append(&mut found_changes)
            }
        }
    }

    depedency_changes
}

fn render_update_difference(dependency_changes: &Vec<DependencyChange>) {
    for dependency_change in dependency_changes.iter() {
        match dependency_change {
            DependencyChange::Added(package_id) => {
                println!("Added {} v{}", package_id.name(), package_id.version());
            }
            DependencyChange::Removed(package_id) => {
                println!("Removed {} v{}", package_id.name(), package_id.version());
            }
            DependencyChange::Updated { from, to } => {
                println!(
                    "Updated {} from v{} to v{}",
                    from.name(),
                    from.version(),
                    to.version()
                );
            }
        }
    }
}
