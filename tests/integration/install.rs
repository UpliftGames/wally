use super::temp_project::TempProject;
use libwally::{Args, GlobalOptions, InstallSubcommand, Subcommand};
use std::path::Path;

#[test]
fn minimal() {
    let project = run_install_test("minimal");
    assert_dir_snapshot!(project.path());
}

#[test]
fn dependency_with_types() {
    let project = run_install_test("dependency-with-types");
    assert_dir_snapshot!(project.path());
}

#[test]
fn one_dependency() {
    let project = run_install_test("one-dependency");
    assert_dir_snapshot!(project.path());
}

#[test]
fn transitive_dependency() {
    let project = run_install_test("transitive-dependency");
    assert_dir_snapshot!(project.path());
}

#[test]
fn private_with_public_dependency() {
    let project = run_install_test("private-with-public-dependency");
    assert_dir_snapshot!(project.path());
}

#[test]
fn dev_dependency() {
    let project = run_install_test("dev-dependency");
    assert_dir_snapshot!(project.path());
}

#[test]
fn dev_dependency_also_required_as_non_dev() {
    let project = run_install_test("dev-dependency-also-required-as-non-dev");
    assert_dir_snapshot!(project.path());
}

#[test]
fn cross_realm_dependency() {
    let project = run_install_test("cross-realm-dependency");
    assert_dir_snapshot!(project.path());
}

#[test]
fn cross_realm_explicit_dependency() {
    let project = run_install_test("cross-realm-explicit-dependency");
    assert_dir_snapshot!(project.path());
}

#[test]
fn manifest_links() {
    let project = run_install_test("manifest-links");
    assert_dir_snapshot!(project.path());
}

#[test]
fn locked_pass() {
    let result = run_locked_install("diamond-graph/root/latest");

    assert!(result.is_ok(), "Should pass without any problems");
}

#[test]
fn locked_catches_dated_packages() {
    let result = run_locked_install("diamond-graph/root/dated");
    assert!(result.is_err(), "Should fail!");
}

fn run_locked_install(name: &str) -> Result<(), anyhow::Error> {
    let source_project =
        Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/test-projects",)).join(name);

    let project = TempProject::new(&source_project).unwrap();

    Args {
        global: GlobalOptions {
            test_registry: true,
            ..Default::default()
        },
        subcommand: Subcommand::Install(InstallSubcommand {
            project_path: project.path().to_owned(),
            locked: true,
        }),
    }
    .run()
}

fn run_install_test(name: &str) -> TempProject {
    let source_project =
        Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/test-projects",)).join(name);

    let project = TempProject::new(&source_project).unwrap();

    let args = Args {
        global: GlobalOptions {
            test_registry: true,
            ..Default::default()
        },
        subcommand: Subcommand::Install(InstallSubcommand {
            project_path: project.path().to_owned(),
            locked: false,
        }),
    };

    args.run().unwrap();
    
    project
}
