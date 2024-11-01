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
    let added_packages = new_dependencies.difference(old_dependencies);
    let removed_packages = old_dependencies.difference(new_dependencies);
    let changed_dependencies: BTreeSet<&PackageName> = added_packages
        .clone()
        .chain(removed_packages.clone())
        .map(|package| package.name())
        .collect();

    let mut dependency = Vec::new();

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

                if package_added.gt(package_removed) {
                    dependency.push(DependencyChange::Updated {
                        from: package_removed.clone(),
                        to: package_added.clone(),
                    });
                } else {
                    dependency.push(DependencyChange::Downgrade {
                        from: package_added.clone(),
                        to: package_removed.clone(),
                    });
                }
            }
            (0, 1) => {
                // We know for certain that there is only one item in the iterator.
                let package_removed = matching_packages_removed.last().unwrap();
                dependency.push(DependencyChange::Removed(package_removed.clone()));
            }
            (1, 0) => {
                // We know for certain that there is only one item in the iterator.
                let package_added = matching_packages_added.last().unwrap();
                dependency.push(DependencyChange::Added(package_added.clone()));
            }
            (0, 0) => panic!("Impossible for the package name {} to not be removed or added if found in earlier.", dependency_name),
            (_, _) => {
                dependency.extend(matching_packages_added.map(|package| DependencyChange::Added(package.clone())));
                dependency.extend(matching_packages_removed.map(|package| DependencyChange::Removed(package.clone())));
            }
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
