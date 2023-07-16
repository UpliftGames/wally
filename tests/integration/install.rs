use super::temp_project::TempProject;
use libwally::{Args, GlobalOptions, InstallSubcommand, Subcommand};
use std::path::Path;

#[test]
fn minimal() {
    assert_project_snapshot!(run_test("minimal"));
}

#[test]
fn one_dependency() {
    assert_project_snapshot!(run_test("one-dependency"));
}

#[test]
fn transitive_dependency() {
    assert_project_snapshot!(run_test("transitive-dependency"));
}

#[test]
fn private_with_public_dependency() {
    assert_project_snapshot!(run_test("private-with-public-dependency"));
}

#[test]
fn dev_dependency() {
    assert_project_snapshot!(run_test("dev-dependency"));
}

#[test]
fn dev_dependency_also_required_as_non_dev() {
    assert_project_snapshot!(run_test("dev-dependency-also-required-as-non-dev"));
}

#[test]
fn cross_realm_dependency() {
    assert_project_snapshot!(run_test("cross-realm-dependency"));
}

#[test]
fn cross_realm_explicit_dependency() {
    assert_project_snapshot!(run_test("cross-realm-explicit-dependency"));
}

fn run_test(name: &str) -> TempProject {
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
        }),
    };

    args.run().unwrap();

    project
}
