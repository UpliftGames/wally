use crate::{
    manifest::{Manifest, Realm},
    package_req::PackageReq,
    package_source::{PackageSource, PackageSourceMap, Registry, TestRegistry},
    GlobalOptions,
};
use anyhow::Context;
use fs_err as fs;
use semver::VersionReq;
use std::{cmp::Ordering, path::PathBuf};
use structopt::StructOpt;

use super::utils::PackageSpec;

#[derive(Debug, StructOpt)]
pub struct AddSubcommand {
    /// Path to the project to add dependencies.
    #[structopt(long = "project-path", default_value = ".")]
    pub project_path: PathBuf,

    /// What realm (dev, server, shared) you wish to add your dependency.
    #[structopt(long = "realm", default_value = "shared")]
    pub what_realm: Realm,

    /// Desired dependencies to add.
    /// If it's a named dependency, it will pick the latest version.
    pub dependencies: Vec<PackageSpec>,
}

impl Realm {
    fn as_table_name(&self) -> &str {
        match self {
            Realm::Server => "server-dependencies",
            Realm::Shared => "dependencies",
            Realm::Dev => "dev-dependencies",
        }
    }
}

impl AddSubcommand {
    pub fn run(self, global: GlobalOptions) -> anyhow::Result<()> {
        if !self.dependencies.is_empty() {
            anyhow::bail!("One more or more dependencies should of been specified.")
        }

        let manifest =
            Manifest::load(&self.project_path).context("Expected a valid wally.toml file.")?;

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

        let mut manifest = String::from_utf8(fs::read(self.project_path.join("wally.toml"))?)?
            .parse::<toml_edit::Document>()?;

        self.update_manifest(&mut manifest, package_sources)?;

        fs::write(self.project_path.join("wally.toml"), manifest.to_string())?;

        Ok(())
    }

    fn update_manifest(
        &self,
        manifest: &mut toml_edit::Document,
        package_sources: PackageSourceMap,
    ) -> anyhow::Result<()> {
        let table_name = self.what_realm.as_table_name();
        let table = match manifest
            .as_table_mut()
            .entry(table_name)
            .or_insert(toml_edit::table())
        {
            toml_edit::Item::Table(table) => table,
            entry @ _ => anyhow::bail!(
                "Found unexpectedly {} found for {}!",
                entry.type_name(),
                table_name
            ),
        };
        let was_lexicographically_sorted = is_table_lexicographically_sorted(table);
        for package_spec in &self.dependencies {
            let alias = match &package_spec {
                PackageSpec::Named(named) => named.name().to_owned(),
                PackageSpec::Required(required) => required.name().name().to_owned(),
            }
            // Luau does not do kebab-casing.
            .replace("-", "_");

            // Make sure that the package actually exists and convert into a requirement to place in the manifest.
            let package_req = match package_spec {
                PackageSpec::Named(named) => {
                    let query = PackageReq::new(named.clone(), VersionReq::STAR);

                    let mut packages = package_sources.search_for(&query)?.1;
                    packages.sort_by(|a, b| a.package_id().version().cmp(b.package_id().version()));
                    let latest_package = packages.last().unwrap().package_id();
                    let latest_version = latest_package.version();

                    PackageReq::new(
                        named.clone(),
                        VersionReq {
                            comparators: vec![semver::Comparator {
                                op: semver::Op::Caret,
                                major: latest_version.major,
                                minor: Some(latest_version.minor),
                                patch: Some(latest_version.patch),
                                pre: latest_version.pre.clone(),
                            }],
                        },
                    )
                }
                PackageSpec::Required(required) => {
                    let _ = package_sources.search_for(&required)?;
                    required.clone()
                }
            };

            if let Some(_) = table.get(&alias) {
                anyhow::bail!(
                    "The alias {} already exists in {}! Stopped to prevent overriding.",
                    alias,
                    self.what_realm
                );
            }

            table.insert(&alias, toml_edit::value(package_req.to_string()));
        }

        if was_lexicographically_sorted {
            table.sort_values_by(|key_a, _, key_b, _| compare_key_lexicographically(key_a, key_b))
        }

        Ok(())
    }
}

fn is_table_lexicographically_sorted(table: &toml_edit::Table) -> bool {
    let length = table.len();
    if length <= 1 {
        true
    } else {
        let mut index = 1;
        let items = table.get_values();

        while index < length - 1 {
            let last_key = &items[index - 1].0;
            let current_key = &items[index].0;

            match compare_list_of_keys_lexicographically(last_key, current_key) {
                Ordering::Greater => return false,
                // TODO: I don't think it's possible for it to be equal?
                // Should be fine, TM.
                Ordering::Less | Ordering::Equal => {}
            };

            index += 1
        }

        true
    }
}

fn compare_list_of_keys_lexicographically(
    a: &[&toml_edit::Key],
    b: &[&toml_edit::Key],
) -> Ordering {
    a.iter()
        .zip(b.iter())
        // TODO: this is very anglocentric, maybe fix?
        .find_map(|(a, b)| match compare_key_lexicographically(a, b) {
            Ordering::Equal => None,
            ordering @ _ => Some(ordering),
        })
        .unwrap_or_else(|| a.len().cmp(&b.len()))
}

fn compare_key_lexicographically(a: &toml_edit::Key, b: &toml_edit::Key) -> Ordering {
    a.to_lowercase().cmp(&b.to_lowercase())
}
