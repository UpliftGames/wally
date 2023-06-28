use anyhow::Context;
use crossterm::style::{Color, SetForegroundColor};
use fs_err as fs;
use std::{path::PathBuf, str::FromStr};
use structopt::StructOpt;

use super::utils::as_table_name;
use crate::manifest::{Manifest, Realm};

const REALMS: [Realm; 3] = [Realm::Server, Realm::Shared, Realm::Dev];

#[derive(StructOpt, Debug)]
pub struct RemoveSubcommand {
    /// Path to the project to publish.
    #[structopt(long = "project-path", default_value = ".")]
    pub project_path: PathBuf,

    /// Dependencies that you want to get rid of.
    /// You can either just specify the alias (e.g, "roact") which will delete any instances of "roact"
    /// Or, you can also specify the realm to delete from (e.g "dev:roact").
    pub target_dependencies: Vec<AliasTarget>,
}

impl RemoveSubcommand {
    pub fn run(self) -> anyhow::Result<()> {
        if self.target_dependencies.is_empty() {
            anyhow::bail!("Specified no dependencies to remove!")
        }

        Manifest::load(&self.project_path).context("Expected a valid wally.toml file.")?;

        let mut manifest: toml_edit::Document =
            String::from_utf8(fs::read(self.project_path.join("wally.toml"))?)?.parse()?;

        for target in self.target_dependencies {
            if let Some(realm) = target.realm {
                if let Some(entry) = remove_dependency_from_realm(realm, &mut manifest, &target)? {
                    write_removal(entry, &target);
                }
            } else {
                for realm in REALMS {
                    if let Some(entry) =
                        remove_dependency_from_realm(realm, &mut manifest, &target)?
                    {
                        write_removal(entry, &target);
                    }
                }
            }
        }

        fs::write(self.project_path.join("wally.toml"), manifest.to_string())?;

        println!(
            "{}   Finished{} removing target dependencies.",
            SetForegroundColor(Color::Green),
            SetForegroundColor(Color::Reset)
        );

        Ok(())
    }
}

fn write_removal(entry: toml_edit::Item, alias_target: &AliasTarget) {
    match entry {
        toml_edit::Item::Value(value) => {
            let package_req = value.as_str().unwrap();
            println!(
                "{}    Removed {}\"{}\" ({}) from {}.",
                SetForegroundColor(Color::DarkRed),
                SetForegroundColor(Color::Reset),
                alias_target.name,
                package_req,
                write_alias_target_realm(alias_target)
            )
        }
        _ => unreachable!("It shouldn't be possible for the table to be of any other type."),
    }
}

fn write_alias_target_realm(alias_target: &AliasTarget) -> &str {
    if let Some(realm) = alias_target.realm {
        match realm {
            Realm::Server => "server realm",
            Realm::Shared => "shared realm",
            Realm::Dev => "dev realm",
        }
    } else {
        "all realms"
    }
}

fn remove_dependency_from_realm(
    realm: Realm,
    manifest: &mut toml_edit::Document,
    target: &AliasTarget,
) -> anyhow::Result<Option<toml_edit::Item>> {
    let table_name = as_table_name(&realm);

    manifest
        .as_table_mut()
        .get_mut(table_name)
        .map_or(Ok(None), |value| {
            let table = match value {
                toml_edit::Item::Table(table) => table,
                entry => anyhow::bail!(
                    "Found unexpectedly {} found for {}!",
                    entry.type_name(),
                    table_name
                ),
            };

            Ok(table.remove(&target.name))
        })
}

#[derive(Debug)]
pub struct AliasTarget {
    realm: Option<Realm>,
    name: String,
}

impl FromStr for AliasTarget {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (realm, name) = match s.split_once(':') {
            Some((realm, name)) => (Some(Realm::from_str(realm)?), name.to_string()),
            None => (None, s.to_string()),
        };

        if !valid_identifier(name.as_str()) {
            anyhow::bail!("Expected target '{}' to be alphanumeric.")
        }

        Ok(AliasTarget { realm, name })
    }
}

fn valid_identifier(name: &str) -> bool {
    name.chars().all(|c| c.is_alphanumeric())
}
