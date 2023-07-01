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

    /// Packages that you want to get rid of.
    /// You can either just specify the alias (e.g, "roact") which will delete any instances of "roact"
    /// Or, you can also specify the realm to delete from (e.g "dev:roact").
    pub packages: Vec<PackageParam>,
}

impl RemoveSubcommand {
    pub fn run(self) -> anyhow::Result<()> {
        if self.packages.is_empty() {
            anyhow::bail!("Specified no dependencies to remove!")
        }

        Manifest::load(&self.project_path).context("Expected a valid wally.toml file.")?;

        let mut manifest_document = {
            let content = fs::read_to_string(self.project_path.join("wally.toml"))?;
            content.parse::<toml_edit::Document>()?
        };
        let manifest = manifest_document.as_table_mut();

        for target in self.packages {
            if let Some(realm) = target.realm {
                if let Some(entry) = remove_dependency_from_realm(realm, manifest, &target)? {
                    write_removal(entry, &target, realm);
                } else {
                    println!(
                        "{}    Missed{} {} in {}",
                        SetForegroundColor(Color::DarkYellow),
                        SetForegroundColor(Color::Reset),
                        target.name,
                        realm
                    );
                }
            } else {
                let present_in_dev = is_alias_present(Realm::Dev, &target.name, manifest);
                let present_in_shared = is_alias_present(Realm::Shared, &target.name, manifest);
                let present_in_server = is_alias_present(Realm::Server, &target.name, manifest);

                let realm = match (present_in_shared, present_in_server, present_in_dev) {
                    (true, false, false) => Realm::Shared,
                    (false, true, false) => Realm::Server,
                    (false, false, true) => Realm::Dev,
                    (false, false, false) => {
                        println!(
                            "{}    Missed{} {}",
                            SetForegroundColor(Color::DarkYellow),
                            SetForegroundColor(Color::Reset),
                            target.name
                        );
                        continue;
                    }
                    (_, _, _) => {
                        anyhow::bail!(
                            "{} Error{} the alias {} is ambiguous as it is present in:\n{}{}{}Specify which realm you want to remove! (e.g 'shared:{}'). ",
                            SetForegroundColor(Color::DarkRed),
                            SetForegroundColor(Color::Reset),
                            target.name,
                            if present_in_shared { "Shared\n" } else { "" },
                            if present_in_server { "Server\n" } else { "" },
                            if present_in_dev { "Dev\n" } else { "" },
                            target.name,
                        )
                    }
                };

                let entry = remove_dependency_from_realm(realm, manifest, &target)?.unwrap();
                write_removal(entry, &target, realm);
            }
        }

        for realm in REALMS {
            let table_name = as_table_name(&realm);
            if manifest
                .get(table_name)
                .map_or(false, |x| x.as_table().unwrap().is_empty())
            {
                manifest.remove(table_name);
            }
        }

        fs::write(self.project_path.join("wally.toml"), manifest.to_string())?;

        println!(
            "{}   Finished{} removing target dependencies.",
            SetForegroundColor(Color::DarkGreen),
            SetForegroundColor(Color::Reset)
        );

        Ok(())
    }
}

fn write_removal(entry: toml_edit::Item, alias_target: &PackageParam, which_realm: Realm) {
    match entry {
        toml_edit::Item::Value(value) => {
            let package_req = value.as_str().unwrap();
            println!(
                "{}    Removed {}\"{}\" ({}) from the {} realm.",
                SetForegroundColor(Color::DarkRed),
                SetForegroundColor(Color::Reset),
                alias_target.name,
                package_req,
                which_realm
            )
        }
        _ => unreachable!("It shouldn't be possible for the table to be of any other type."),
    }
}

fn remove_dependency_from_realm(
    realm: Realm,
    manifest: &mut toml_edit::Table,
    target: &PackageParam,
) -> anyhow::Result<Option<toml_edit::Item>> {
    let table_name = as_table_name(&realm);

    manifest.get_mut(table_name).map_or(Ok(None), |value| {
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

fn is_alias_present(realm: Realm, alias: &str, manifest: &toml_edit::Table) -> bool {
    let table_name = as_table_name(&realm);
    manifest
        .get(table_name)
        .map(|table| table.get(alias).is_some())
        .unwrap_or(false)
}

#[derive(Debug)]
pub struct PackageParam {
    realm: Option<Realm>,
    name: String,
}

impl FromStr for PackageParam {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (realm, name) = match s.split_once(':') {
            Some((realm, name)) => (Some(Realm::from_str(realm)?), name.to_string()),
            None => (None, s.to_string()),
        };

        if !valid_identifier(name.as_str()) {
            anyhow::bail!("Expected target '{}' to be alphanumeric.")
        }

        Ok(PackageParam { realm, name })
    }
}

fn valid_identifier(name: &str) -> bool {
    name.chars().all(|c| c.is_alphanumeric())
}
