use crate::{temp_project::TempProject, util::read_path};
use fs_err as fs;
use insta::assert_snapshot;
use libwally::{
    lockfile::Lockfile,
    manifest::{Manifest, Package},
    package_id::PackageId,
    package_name::PackageName,
    package_req::PackageReq,
    Args, GlobalOptions, InstallSubcommand, PackageSpec, Subcommand, UpdateSubcommand,
};
use semver::Version;
use std::{
    convert::{TryFrom, TryInto},
    path::Path,
    str::FromStr,
};

#[test]
fn test_test() {
    let source_project = Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/test-projects/diamond-graph/root/0.1.0"
    ));

    let project = TempProject::new(&source_project).unwrap();
    run_download(&project);

    let args = Args {
        global: GlobalOptions {
            test_registry: true,
            ..Default::default()
        },
        subcommand: Subcommand::Update(UpdateSubcommand {
            project_path: project.path().to_owned(),
            package_specs: Vec::new(),
        }),
    };

    args.run().unwrap();
}

#[test]
fn generate_new_lockfile_if_missing() {
    let source_project = Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/test-projects/diamond-graph/root/0.1.0"
    ));

    let project = TempProject::new(&source_project).unwrap();

    let args = Args {
        global: GlobalOptions {
            test_registry: true,
            ..Default::default()
        },
        subcommand: Subcommand::Update(UpdateSubcommand {
            project_path: project.path().to_owned(),
            package_specs: Vec::new(),
        }),
    };

    let result = args.run();

    assert!(result.is_ok(), "It should of ran without any errors.");

    assert!(
        fs::metadata(project.path().join("wally.lock"))
            .unwrap()
            .is_file(),
        "It should've of generated a new wally.lock file."
    )
}

#[test]
fn do_nothing() {
    let source_project = Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/test-projects/diamond-graph/root/0.1.0"
    ));

    let project = TempProject::new(&source_project).unwrap();

    run_download(&project);

    Args {
        global: GlobalOptions {
            test_registry: true,
            ..Default::default()
        },
        subcommand: Subcommand::Update(UpdateSubcommand {
            project_path: project.path().to_owned(),
            package_specs: Vec::new(),
        }),
    }
    .run()
    .unwrap();

    let first_snapshot = read_path(project.path()).unwrap();

    run_update(&project);

    let second_snapshot = read_path(project.path()).unwrap();

    assert!(
        first_snapshot == second_snapshot,
        "Update is idiompotent assuming the same state."
    );
}

#[test]
fn update_all_dependencies() {
    let source_project = Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/test-projects/diamond-graph/root/0.1.0"
    ));

    let project = TempProject::new(&source_project).unwrap();

    let mut manifest = Manifest::load(project.path()).unwrap();

    run_download(&project);

    let first_lockfile = fs::read_to_string(project.path().join("wally.lock")).unwrap();
    assert_snapshot!(first_lockfile);

    manifest.server_dependencies.insert(
        "A".into(),
        PackageReq::from_str("diamond-graph/direct-dependency-a@0.1.0".into()).unwrap(),
    );

    Args {
        global: GlobalOptions {
            test_registry: true,
            ..Default::default()
        },
        subcommand: Subcommand::Update(UpdateSubcommand {
            project_path: project.path().to_owned(),
            package_specs: Vec::new(),
        }),
    }
    .run()
    .unwrap();

    let second_lockfile = fs::read_to_string(project.path().join("wally.lock")).unwrap();
    assert_snapshot!(second_lockfile);
}

#[test]
fn updated_named_dependency() {
    let source_project = Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/test-projects/diamond-graph/root/0.1.0"
    ));

    let project = TempProject::new(&source_project).unwrap();

    let mut manifest = Manifest::load(project.path()).unwrap();

    run_download(&project);

    let first_lockfile = fs::read_to_string(project.path().join("wally.lock")).unwrap();
    assert_snapshot!(first_lockfile);

    manifest.server_dependencies.insert(
        "A".into(),
        PackageReq::from_str("diamond-graph/direct-dependency-a@0.1.0".into()).unwrap(),
    );

    Args {
        global: GlobalOptions {
            test_registry: true,
            ..Default::default()
        },
        subcommand: Subcommand::Update(UpdateSubcommand {
            project_path: project.path().to_owned(),
            package_specs: vec![PackageSpec::Named(
                PackageName::new("diamond-graph", "direct-dependency-a").unwrap(),
            )],
        }),
    }
    .run()
    .unwrap();

    let second_lockfile = fs::read_to_string(project.path().join("wally.lock")).unwrap();
    assert_snapshot!(second_lockfile);
}

#[test]
fn update_named_dependency() {
    let source_project = Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/test-projects/diamond-graph/root/0.1.0"
    ));

    let project = TempProject::new(&source_project).unwrap();

    let mut manifest = Manifest::load(project.path()).unwrap();

    run_download(&project);

    let first_lockfile = fs::read_to_string(project.path().join("wally.lock")).unwrap();
    assert_snapshot!(first_lockfile);

    manifest.server_dependencies.insert(
        "A".into(),
        PackageReq::from_str("diamond-graph/direct-dependency-a@0.1.0".into()).unwrap(),
    );

    run_update_with_specs(
        &project,
        vec![PackageSpec::Required(
            PackageReq::from_str("diamond-graph/direct-dependency-a@0.1.0".into()).unwrap(),
        )],
    );

    let second_lockfile = fs::read_to_string(project.path().join("wally.lock")).unwrap();
    assert_snapshot!(second_lockfile);
}

fn update_list_of_specs() {}

fn update_should_download_new_dependences() {}

fn run_update(project: &TempProject) {
    run_update_with_specs(project, Vec::new())
}

fn run_update_with_specs(project: &TempProject, specs: Vec<PackageSpec>) {
    Args {
        global: GlobalOptions {
            test_registry: true,
            ..Default::default()
        },
        subcommand: Subcommand::Update(UpdateSubcommand {
            project_path: project.path().to_owned(),
            package_specs: specs,
        }),
    }
    .run()
    .unwrap();
}

fn run_download(project: &TempProject) {
    Args {
        global: GlobalOptions {
            test_registry: true,
            ..Default::default()
        },
        subcommand: Subcommand::Install(InstallSubcommand {
            project_path: project.path().to_owned(),
        }),
    }
    .run()
    .unwrap();
}
