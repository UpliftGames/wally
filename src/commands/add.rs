use crate::{
    manifest::{Manifest, Realm},
    package_name::PackageName,
    package_req::PackageReq,
    package_source::{PackageSource, PackageSourceMap, Registry, TestRegistry},
    GlobalOptions,
};
use anyhow::Context;
use crossterm::style::{Color, SetForegroundColor};
use fs_err as fs;
use semver::{Version, VersionReq};
use std::{cmp::Ordering, path::PathBuf, str::FromStr};
use structopt::StructOpt;

use super::utils::{as_table_name, PackageSpec};

#[derive(Debug, StructOpt)]
pub struct AddSubcommand {
    /// Path to the project to add dependencies.
    #[structopt(long = "project-path", default_value = ".")]
    pub project_path: PathBuf,

    /// What realm (dev, server, shared) you wish to add your dependency.
    #[structopt(long = "realm", default_value = "shared")]
    pub what_realm: Realm,

    /// Desired packages to add.
    /// By default, the alias is the package's name.
    /// If the version is omitted, it will pick the latest version.
    pub packages: Vec<PackageParam>,
}

impl AddSubcommand {
    pub fn run(self, global: GlobalOptions) -> anyhow::Result<()> {
        if self.packages.is_empty() {
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

        let mut manifest: toml_edit::Document =
            String::from_utf8(fs::read(self.project_path.join("wally.toml"))?)?.parse()?;

        let table_name = as_table_name(&self.what_realm);
        let table = match manifest
            .as_table_mut()
            .entry(table_name)
            .or_insert(toml_edit::table())
        {
            toml_edit::Item::Table(table) => table,
            entry => anyhow::bail!(
                "Found unexpectedly {} found for {}!",
                entry.type_name(),
                table_name
            ),
        };

        let was_lexicographically_sorted = is_table_lexicographically_sorted(table);

        for desired_package in &self.packages {
            let alias = desired_package
                .alias
                .as_deref()
                .unwrap_or(desired_package.spec.name())
                // Luau does not do kebab-casing.
                .replace('-', "_");

            // Make sure that the package actually exists and convert into a requirement to place in the manifest.
            let package_req = match &desired_package.spec {
                PackageSpec::Named(named) => {
                    let query = PackageReq::new(named.clone(), VersionReq::STAR);

                    let mut packages = package_sources.search_for(&query)?.1;
                    packages.sort_by(|a, b| a.package_id().version().cmp(b.package_id().version()));
                    let latest_package = packages.last().unwrap().package_id();
                    let latest_version = latest_package.version();

                    into_carot_req(named, latest_version)
                }
                PackageSpec::Required(required) => {
                    let matches = package_sources.search_for(required)?.1;
                    if matches.is_empty() {
                        anyhow::bail!(
                            "Could not find a package from any sources that matched {}!",
                            required
                        )
                    }

                    required.clone()
                }
            };

            if table.get(&alias).is_some() {
                anyhow::bail!(
                    "The alias {} already exists in {}! Stopped to prevent overriding.",
                    alias,
                    self.what_realm
                );
            }

            println!(
                "{}      Added{} {} as {}.",
                SetForegroundColor(Color::DarkGreen),
                SetForegroundColor(Color::Reset),
                package_req,
                alias
            );

            table.insert(&alias, toml_edit::value(package_req.to_string()));
        }

        if was_lexicographically_sorted {
            table.sort_values_by(|key_a, _, key_b, _| compare_key_lexicographically(key_a, key_b));

            println!(
                "{}     Sorted{} dependencies alphabetically.",
                SetForegroundColor(Color::DarkGreen),
                SetForegroundColor(Color::Reset)
            );
        }

        fs::write(self.project_path.join("wally.toml"), manifest.to_string())?;
        println!(
            "{}   Finished{} updated manifest with new dependencies!",
            SetForegroundColor(Color::DarkGreen),
            SetForegroundColor(Color::Reset)
        );

        Ok(())
    }
}

#[derive(Debug)]
pub struct PackageParam {
    // TODO: merge Alias and use it here instead.
    pub alias: Option<String>,
    pub spec: PackageSpec,
}

impl FromStr for PackageParam {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (alias, spec) = match s.split_once(':') {
            Some((alias, spec)) => (Some(alias.to_owned()), spec.parse()?),
            None => {
                let spec = s.parse()?;
                (None, spec)
            }
        };

        Ok(PackageParam { alias, spec })
    }
}

fn into_carot_req(named: &PackageName, latest_version: &Version) -> PackageReq {
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

fn is_table_lexicographically_sorted(table: &toml_edit::Table) -> bool {
    let items = table.get_values();

    let behind = items.iter().map(|x| &x.0);
    let leading = behind.clone().skip(1);

    // Verifying that it's in ascending order by checking if for all of n to total minus one, k[n] <= k[n + 1] holds!
    behind.zip(leading).all(|(prior_key, following_key)| {
        match compare_list_of_keys_lexicographically(prior_key, following_key) {
            Ordering::Greater => false,
            Ordering::Less => true,
            Ordering::Equal => unreachable!("Impossible for duplicate keys to exist!"),
        }
    })
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
            ordering => Some(ordering),
        })
        .unwrap_or_else(|| a.len().cmp(&b.len()))
}

fn compare_key_lexicographically(a: &toml_edit::Key, b: &toml_edit::Key) -> Ordering {
    a.to_lowercase().cmp(&b.to_lowercase())
}
