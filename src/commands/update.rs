use std::collections::BTreeSet;

use std::path::PathBuf;
use std::str::FromStr;
use std::time::Duration;

use crate::installation::InstallationContext;
use crate::lockfile::Lockfile;
use crate::package_id::PackageId;
use crate::package_name::PackageName;
use crate::package_req::PackageReq;
use crate::package_source::{PackageSource, PackageSourceMap, Registry, TestRegistry};
use crate::resolution::resolve_workspace;
use crate::workspace::Workspace;
use crate::GlobalOptions;
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
        let workspace_root = Workspace::discover_root(&self.project_path)?;
        let workspace = Workspace::load(&workspace_root)?;

        let lockfile = match Lockfile::load(workspace.root())? {
            Some(lockfile) => lockfile,
            None => Lockfile::from_workspace(&workspace),
        };

        let default_registry: Box<PackageSource> = if global.test_registry {
            Box::new(PackageSource::TestRegistry(TestRegistry::new(
                workspace.registry(),
            )))
        } else {
            Box::new(PackageSource::Registry(Registry::from_registry_spec(
                workspace.registry(),
            )?))
        };

        let mut package_sources = PackageSourceMap::new(default_registry);
        package_sources.add_fallbacks()?;

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

        let resolved_graph = resolve_workspace(&workspace, &try_to_use, &package_sources)?;

        let member_count = workspace.members().len();
        let total_deps = resolved_graph.activated.len();
        let external_deps = total_deps.saturating_sub(member_count);

        progress.println(format!(
            "{}   Resolved {}{} total dependencies",
            SetForegroundColor(Color::DarkGreen),
            SetForegroundColor(Color::Reset),
            external_deps
        ));

        progress.enable_steady_tick(Duration::from_millis(100));
        progress.suspend(|| {
            let dependency_changes = generate_dependency_changes(
                &lockfile.as_ids().collect(),
                &resolved_graph.activated,
            );
            render_update_difference(&dependency_changes, &mut std::io::stdout()).unwrap();
        });

        Lockfile::from_resolve(&resolved_graph, Some(workspace.root()))
            .save(workspace.root())?;

        progress.println(format!(
            "{}    Updated {}lockfile",
            SetForegroundColor(Color::DarkGreen),
            SetForegroundColor(Color::Reset)
        ));

        let root_package_ids: BTreeSet<_> = workspace
            .members()
            .values()
            .map(|m| m.package_id())
            .collect();

        let installation_context = InstallationContext::new(
            workspace.root(),
            workspace.place().shared_packages.clone(),
            workspace.place().server_packages.clone(),
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

        installation_context.install(package_sources, root_package_ids, resolved_graph)?;

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
