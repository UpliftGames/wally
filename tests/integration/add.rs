use fs_err as fs;
use insta::assert_snapshot;
use libwally::{
    manifest::Realm, package_name::PackageName, package_req::PackageReq, AddSubcommand, Args,
    GlobalOptions, PackageSpec, Subcommand,
};
use std::{path::Path, str::FromStr};

use crate::temp_project::TempProject;

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
            dependencies: vec![PackageSpec::Named(
                PackageName::from_str("biff/minimal").unwrap(),
            )],
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
            dependencies: vec![PackageSpec::Required(
                // It has to be exact!
                PackageReq::from_str("biff/minimal@=0.1.0").unwrap(),
            )],
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
            dependencies: vec![PackageSpec::Required(
                PackageReq::from_str("biff/minimal@1.0.0").unwrap(),
            )],
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
            dependencies: vec![PackageSpec::Named(
                PackageName::from_str("biff/i-dont-exist").unwrap(),
            )],
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
            dependencies: vec![PackageSpec::Named(
                PackageName::from_str("biff/minimal").unwrap(),
            )],
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
            dependencies: vec![
                PackageSpec::Named(PackageName::from_str("biff/minimal").unwrap()),
                PackageSpec::Required(
                    PackageReq::from_str("biff/transitive-dependency@=0.1.0").unwrap(),
                ),
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
            dependencies: vec![
                PackageSpec::Named(PackageName::from_str("biff/minimal").unwrap()),
                PackageSpec::Named(
                    PackageName::from_str("diamond-graph/direct-dependency-b").unwrap(),
                ),
                PackageSpec::Named(
                    PackageName::from_str("diamond-graph/direct-dependency-a").unwrap(),
                ),
            ],
        }),
    };

    args.run().unwrap();
    snapshot_manifest(&project);
}

fn snapshot_manifest(project: &TempProject) {
    assert_snapshot!(fs::read_to_string(project.path().join("wally.toml")).unwrap())
}

fn sorted_project() -> TempProject {
    TempProject::new(Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/test-projects/sorted-dependencies"
    )))
    .unwrap()
}

fn unsorted_project() -> TempProject {
    TempProject::new(Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/test-projects/unsorted-dependencies"
    )))
    .unwrap()
}
