use crate::{package_id::PackageId, package_name::PackageName};
use crossterm::style::{Color, SetForegroundColor};
use serde::Serialize;
use std::{collections::BTreeSet, io::Write};

#[derive(Debug, Eq, PartialEq, Serialize)]
pub(crate) enum DependencyChange {
    Added(PackageId),
    Removed(PackageId),
    Upgraded { from: PackageId, to: PackageId },
    Downgraded { from: PackageId, to: PackageId },
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
                        DependencyChange::Upgraded { from: old, to: new }
                    }
                    (Some(old), Some(new)) => DependencyChange::Downgraded { from: old, to: new },

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

        return Ok(());
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
            DependencyChange::Upgraded { from, to } => writeln!(
                writer,
                "{}    Updated {}{} from v{} to v{}",
                SetForegroundColor(Color::DarkCyan),
                SetForegroundColor(Color::Reset),
                from.name(),
                from.version(),
                to.version()
            ),
            DependencyChange::Downgraded { from, to } => writeln!(
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

#[cfg(test)]
mod test {
    use std::{collections::BTreeSet, str::FromStr};

    use super::{generate_dependency_changes, render_update_difference};
    use insta::assert_snapshot;

    macro_rules! package_id {
        ($package_id:literal) => {
            crate::package_id::PackageId::from_str($package_id).unwrap()
        };
    }

    macro_rules! added_change {
        ($changes:expr, $package_id:literal) => {
            $changes
                .iter()
                .any(|change| *change == super::DependencyChange::Added(package_id!($package_id)))
        };
    }

    macro_rules! removed_change {
        ($changes:expr, $package_id:literal) => {
            $changes
                .iter()
                .any(|change| *change == super::DependencyChange::Removed(package_id!($package_id)))
        };
    }

    macro_rules! upgraded_change {
        ($changes:expr, $from_package_id:literal, $to_package_id:literal) => {
            $changes.iter().any(|change| {
                *change
                    == super::DependencyChange::Upgraded {
                        from: package_id!($from_package_id),
                        to: package_id!($to_package_id),
                    }
            })
        };
    }

    macro_rules! downgraded_change {
        ($changes:expr, $from_package_id:literal, $to_package_id:literal) => {
            $changes.iter().any(|change| {
                *change
                    == super::DependencyChange::Downgraded {
                        from: package_id!($from_package_id),
                        to: package_id!($to_package_id),
                    }
            })
        };
    }

    #[test]
    fn generate_no_changes_if_same() {
        let dependencies = BTreeSet::from([
            package_id!("biff/package-a@1.1.1"),
            package_id!("biff/package-b@1.2.1"),
        ]);

        let changes = generate_dependency_changes(&dependencies, &dependencies);

        assert!(changes.is_empty(), "Expected no changes.")
    }

    #[test]
    fn generate_correct_changes() {
        let old_dependencies = BTreeSet::from([
            package_id!("biff/unchanged-package@1.0.0"),
            package_id!("biff/removed-package@1.2.1"),
            package_id!("biff/upgraded-package@1.2.1"),
            package_id!("biff/downgraded-package@1.2.1"),
        ]);

        let new_dependencies = BTreeSet::from([
            package_id!("biff/unchanged-package@1.0.0"),
            package_id!("biff/added-package@3.1.1"),
            package_id!("biff/upgraded-package@1.2.4"),
            package_id!("biff/downgraded-package@0.0.1"),
        ]);

        let changes = generate_dependency_changes(&old_dependencies, &new_dependencies);

        assert!(!changes.is_empty(), "Expected changes.");
        assert!(changes.len() == 4, "Expected four changes.");
        assert!(
            added_change!(changes, "biff/added-package@3.1.1"),
            "Expected biff/added-package to be added."
        );
        assert!(
            removed_change!(changes, "biff/removed-package@1.2.1"),
            "Expected biff/remove-package to be removed."
        );
        assert!(
            upgraded_change!(
                changes,
                "biff/upgraded-package@1.2.1",
                "biff/upgraded-package@1.2.4"
            ),
            "Expected biff/upgraded-package to be upgraded."
        );
        assert!(
            downgraded_change!(
                changes,
                "biff/downgraded-package@1.2.1",
                "biff/downgraded-package@0.0.1"
            ),
            "Expected biff/downgraded-package to be downgraded."
        );
    }

    #[test]
    fn decompose_upgrades_and_downgrades_if_multiple() {
        let old_dependencies = BTreeSet::from([
            package_id!("biff/package-a@1.0.0"),
            package_id!("biff/package-b@1.0.0"),
        ]);

        let new_dependencies = BTreeSet::from([
            package_id!("biff/package-a@2.0.0"),
            package_id!("biff/package-a@3.0.0"),
            package_id!("biff/package-b@0.5.0"),
            package_id!("biff/package-b@0.5.1"),
        ]);

        let changes = generate_dependency_changes(&old_dependencies, &new_dependencies);

        assert!(!changes.is_empty(), "Expected changes.");
        assert!(changes.len() == 6, "Expected only six changes.");
        assert!(
            removed_change!(changes, "biff/package-a@1.0.0"),
            "Expected package-a@1.0.0 to be removed."
        );
        assert!(
            removed_change!(changes, "biff/package-b@1.0.0"),
            "Expected package-b@1.0.0 to be removed."
        );
        assert!(
            added_change!(changes, "biff/package-a@2.0.0")
                && added_change!(changes, "biff/package-a@3.0.0"),
            "Upgrades decomposed."
        );
        assert!(
            added_change!(changes, "biff/package-b@0.5.0")
                && added_change!(changes, "biff/package-b@0.5.1"),
            "Upgrades decomposed."
        )
    }

    #[test]
    fn snapshot_output_when_no_changes() {
        let changes = Vec::new();

        let mut writer = Vec::new();
        render_update_difference(&changes, &mut writer).unwrap();

        assert_snapshot!(String::from_utf8(writer).unwrap());
    }
}
