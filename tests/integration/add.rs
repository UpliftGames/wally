use libwally::{manifest::Realm, AddSubcommand, Args, GlobalOptions, Subcommand};
use std::path::Path;

use crate::{temp_project::TempProject, util::snapshot_manifest};

#[test]
fn add_named_package() {
    let project = unsorted_project();

    let args = Args {
        global: GlobalOptions {
            test_registry: true,
            ..Default::default()
        },
        subcommand: Subcommand::Add(AddSubcommand {
            project_path: project.path().to_owned(),
            what_realm: Realm::Shared,
            packages: vec!["biff/minimal".parse().unwrap()],
        }),
    };

    args.run().unwrap();
    snapshot_manifest(&project);
}

#[test]
fn add_versioned_package() {
    let project = unsorted_project();

    let args = Args {
        global: GlobalOptions {
            test_registry: true,
            ..Default::default()
        },
        subcommand: Subcommand::Add(AddSubcommand {
            project_path: project.path().to_owned(),
            what_realm: Realm::Shared,
            packages: vec!["biff/minimal@=0.1.0".parse().unwrap()],
        }),
    };

    args.run().unwrap();
    snapshot_manifest(&project);
}

#[test]
fn error_on_invalid_version() {
    let project = unsorted_project();

    let args = Args {
        global: GlobalOptions {
            test_registry: true,
            ..Default::default()
        },
        subcommand: Subcommand::Add(AddSubcommand {
            project_path: project.path().to_owned(),
            what_realm: Realm::Shared,
            packages: vec!["biff/minimal@1.0.0".parse().unwrap()],
        }),
    };

    assert!(
        args.run().is_err(),
        "It should of errored since there's no minimal >=1.0.0!"
    );
}

#[test]
fn error_on_nonexistant_package() {
    let project = unsorted_project();

    let args = Args {
        global: GlobalOptions {
            test_registry: true,
            ..Default::default()
        },
        subcommand: Subcommand::Add(AddSubcommand {
            project_path: project.path().to_owned(),
            what_realm: Realm::Shared,
            packages: vec!["biff/i-dont-exist".parse().unwrap()],
        }),
    };

    assert!(
        args.run().is_err(),
        "It should of errored since there's no 'i-dont-exist'! (please?)"
    );
}

#[test]
fn add_to_target_realm() {
    let project = unsorted_project();

    let args = Args {
        global: GlobalOptions {
            test_registry: true,
            ..Default::default()
        },
        subcommand: Subcommand::Add(AddSubcommand {
            project_path: project.path().to_owned(),
            what_realm: Realm::Server,
            packages: vec!["biff/minimal".parse().unwrap()],
        }),
    };

    args.run().unwrap();
    snapshot_manifest(&project);
}

#[test]
fn add_multiple() {
    let project = unsorted_project();

    let args = Args {
        global: GlobalOptions {
            test_registry: true,
            ..Default::default()
        },
        subcommand: Subcommand::Add(AddSubcommand {
            project_path: project.path().to_owned(),
            what_realm: Realm::Shared,
            packages: vec![
                "biff/minimal".parse().unwrap(),
                "biff/transitive-dependency@=0.1.0".parse().unwrap(),
            ],
        }),
    };

    args.run().unwrap();
    snapshot_manifest(&project);
}

#[test]
fn add_following_lexicographic_sort() {
    let project = sorted_project();

    let args = Args {
        global: GlobalOptions {
            test_registry: true,
            ..Default::default()
        },
        subcommand: Subcommand::Add(AddSubcommand {
            project_path: project.path().to_owned(),
            what_realm: Realm::Shared,
            packages: vec![
                "biff/minimal".parse().unwrap(),
                "diamond-graph/direct-dependency-b".parse().unwrap(),
                "diamond-graph/direct-dependency-a".parse().unwrap(),
            ],
        }),
    };

    args.run().unwrap();
    snapshot_manifest(&project);
}

#[test]
fn specify_alias() {
    let project = unsorted_project();

    let args = Args {
        global: GlobalOptions {
            test_registry: true,
            ..Default::default()
        },
        subcommand: Subcommand::Add(AddSubcommand {
            project_path: project.path().to_owned(),
            what_realm: Realm::Shared,
            packages: vec!["variant:biff/minimal".parse().unwrap()],
        }),
    };

    args.run().unwrap();
    snapshot_manifest(&project);
}

fn sorted_project() -> TempProject {
    open_test_project!("sorted-dependencies")
}

fn unsorted_project() -> TempProject {
    open_test_project!("unsorted-dependencies")
}
