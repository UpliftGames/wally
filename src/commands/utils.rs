use crate::{package_id::PackageId, package_name::PackageName};
use crossterm::style::{Color, SetForegroundColor};
use std::{collections::BTreeSet, io::Write};

pub(crate) enum DependencyChange {
    Added(PackageId),
    Removed(PackageId),
    Updated { from: PackageId, to: PackageId },
    Downgrade { from: PackageId, to: PackageId },
}

pub(crate) fn generate_dependency_changes(
    old_dependencies: &BTreeSet<PackageId>,
    new_dependencies: &BTreeSet<PackageId>,
) -> Vec<DependencyChange> {
    let changed_dependencies: BTreeSet<&PackageName> = old_dependencies
        .symmetric_difference(new_dependencies)
        .map(|changed_package| changed_package.name())
        .collect();

    let mut dependency = Vec::new();

    for changed_dependency_name in changed_dependencies {
        let match_package_ids_by_name =
            |maybe_matching: &&PackageId| maybe_matching.name() == changed_dependency_name;

        let mut old_matches = old_dependencies.iter().filter(match_package_ids_by_name);
        let total_old_matches = old_matches.clone().count();

        let mut new_matches = new_dependencies.iter().filter(match_package_ids_by_name);
        let total_new_matches = new_matches.clone().count();

        // If there's more than one new or old matches, then we do the simple route of listing the exact versions removed/added.
        if total_new_matches > 1 || total_old_matches > 1 {
            dependency
                .extend(old_matches.map(|package| DependencyChange::Removed(package.clone())));
            dependency.extend(new_matches.map(|package| DependencyChange::Added(package.clone())));
        } else {
            // Otherwise, we can try being more specific about what changed.
            dependency.push(
                match (old_matches.next().cloned(), new_matches.next().cloned()) {
                    (Some(old), Some(new)) if old.le(&new) => {
                        DependencyChange::Updated { from: old, to: new }
                    }
                    (Some(old), Some(new)) => DependencyChange::Downgrade { from: old, to: new },

                    // Or, there's been a singular removal/addition.
                    (Some(old), None) => DependencyChange::Removed(old),
                    (None, Some(new)) => DependencyChange::Added(new),
                    (None, None) => panic!(
                        "Impossible for the package name {} to not be removed or added if found \
                     in earlier.",
                        changed_dependency_name
                    ),
                },
            )
        }
    }

    dependency
}

pub(crate) fn render_update_difference(
    dependency_changes: &[DependencyChange],
    writer: &mut impl Write,
) -> anyhow::Result<()> {
    if dependency_changes.is_empty() {
        writeln!(
            writer,
            "{} No Dependency changes{}",
            SetForegroundColor(Color::DarkGreen),
            SetForegroundColor(Color::Reset)
        )?;
    }

    writeln!(
        writer,
        "{} Dependency changes{}",
        SetForegroundColor(Color::DarkGreen),
        SetForegroundColor(Color::Reset)
    )?;

    for dependency_change in dependency_changes {
        match dependency_change {
            DependencyChange::Added(package_id) => writeln!(
                writer,
                "{}      Added {}{} v{}",
                SetForegroundColor(Color::DarkGreen),
                SetForegroundColor(Color::Reset),
                package_id.name(),
                package_id.version()
            ),
            DependencyChange::Removed(package_id) => writeln!(
                writer,
                "{}    Removed {}{} v{}",
                SetForegroundColor(Color::DarkRed),
                SetForegroundColor(Color::Reset),
                package_id.name(),
                package_id.version()
            ),
            DependencyChange::Updated { from, to } => writeln!(
                writer,
                "{}    Updated {}{} from v{} to v{}",
                SetForegroundColor(Color::DarkCyan),
                SetForegroundColor(Color::Reset),
                from.name(),
                from.version(),
                to.version()
            ),
            DependencyChange::Downgrade { from, to } => writeln!(
                writer,
                "{} Downgraded {}{} from v{} to v{}",
                SetForegroundColor(Color::DarkYellow),
                SetForegroundColor(Color::Reset),
                from.name(),
                from.version(),
                to.version()
            ),
        }?
    }

    Ok(())
}
