use libwally::{Args, GlobalOptions, RemoveSubcommand, Subcommand};
use std::path::Path;

use crate::{temp_project::TempProject, util::snapshot_manifest};

#[test]
fn delete_target() {
    let project = open_test_project!("one-dependency");

    let args = Args {
        global: GlobalOptions {
            test_registry: true,
            ..Default::default()
        },
        subcommand: Subcommand::Remove(RemoveSubcommand {
            project_path: project.path().to_owned(),
            packages: vec!["Minimal".parse().unwrap()],
        }),
    };

    args.run().unwrap();
    snapshot_manifest(&project);
}

#[test]
fn should_error_if_alias_is_ambiguous() {
    let project = open_test_project!("duplicate-alias-different-realms");

    let args = Args {
        global: GlobalOptions {
            test_registry: true,
            ..Default::default()
        },
        subcommand: Subcommand::Remove(RemoveSubcommand {
            project_path: project.path().to_owned(),
            packages: vec!["Minimal".parse().unwrap()],
        }),
    };

    let result = args.run();
    assert!(
        result
            .expect_err("Should of errored due to ambiguity.")
            .to_string()
            .contains("ambiguous"),
        "Should contain something about being ambiguous!"
    )
}

#[test]
fn delete_package_from_one_realm() {
    let project = open_test_project!("duplicate-alias-different-realms");

    let args = Args {
        global: GlobalOptions {
            test_registry: true,
            ..Default::default()
        },
        subcommand: Subcommand::Remove(RemoveSubcommand {
            project_path: project.path().to_owned(),
            packages: vec!["server:Minimal".parse().unwrap()],
        }),
    };

    args.run().unwrap();
    snapshot_manifest(&project);
}

#[test]
fn failing_to_match_should_not_error() {
    let project = open_test_project!("one-dependency");

    let args = Args {
        global: GlobalOptions {
            test_registry: true,
            ..Default::default()
        },
        subcommand: Subcommand::Remove(RemoveSubcommand {
            project_path: project.path().to_owned(),
            packages: vec!["it_isnt_there".parse().unwrap()],
        }),
    };

    // Should simply run without panicking.
    args.run().expect("Should run without any error.");
}
