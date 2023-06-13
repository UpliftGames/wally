use crate::temp_project::TempProject;
use fs_err as fs;
use insta::assert_snapshot;
use libwally::{
    package_name::PackageName, package_req::PackageReq, Args, GlobalOptions, PackageSpec,
    Subcommand, UpdateSubcommand,
};
use std::{path::Path, str::FromStr};

#[test]
fn generate_new_lockfile_if_missing() {
    let source_project = Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/test-projects/diamond-graph/root/fresh"
    ));

    let project = TempProject::new(&source_project).unwrap();

    let result = run_update(&project);

    assert!(result.is_ok(), "It should of ran without any errors.");

    assert!(
        fs::metadata(project.path().join("wally.lock"))
            .unwrap()
            .is_file(),
        "It should've of generated a new wally.lock file."
    )
}

#[test]
fn install_new_packages_after_updating() {
    let source_project = Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/test-projects/diamond-graph/root/fresh"
    ));

    let project = TempProject::new(&source_project).unwrap();

    let result = run_update(&project);

    assert!(result.is_ok(), "It should of ran without any errors.");

    assert!(
        fs::metadata(project.path().join("ServerPackages"))
            .unwrap()
            .is_dir(),
        "ServerPackages was made and has its dependencies."
    )
}

#[test]
fn update_all_dependencies() {
    let source_project = Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/test-projects/diamond-graph/root/dated"
    ));

    let project = TempProject::new(&source_project).unwrap();

    run_update(&project).unwrap();

    let lockfile = fs::read_to_string(project.path().join("wally.lock")).unwrap();
    assert_snapshot!(lockfile);
}

#[test]
/// It should update both indirect-dependency-a v0.1.0 and 0.2.0 to v0.1.1 and 0.2.1 respectively
fn update_named_dependency() {
    let source_project = Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/test-projects/diamond-graph/root/dated"
    ));

    let project = TempProject::new(&source_project).unwrap();

    run_update_with_specs(
        &project,
        vec![PackageSpec::Named(
            PackageName::new("diamond-graph", "indirect-dependency-a").unwrap(),
        )],
    )
    .unwrap();

    let lockfile_contents = fs::read_to_string(project.path().join("wally.lock")).unwrap();
    assert_snapshot!(lockfile_contents);
}

#[test]
/// It should only update @0.1.0 instead of the @0.2.0 version.
fn update_required_dependency() {
    let source_project = Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/test-projects/diamond-graph/root/dated"
    ));

    let project = TempProject::new(&source_project).unwrap();

    run_update_with_specs(
        &project,
        vec![PackageSpec::Required(
            PackageReq::from_str("diamond-graph/indirect-dependency-a@0.1.0".into()).unwrap(),
        )],
    )
    .unwrap();

    let lockfile_content = fs::read_to_string(project.path().join("wally.lock")).unwrap();
    assert_snapshot!(lockfile_content);
}

#[test]
/// It should update everything that can be updated, in a round-about way.
fn update_list_of_specs() {
    let source_project = Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/test-projects/diamond-graph/root/dated"
    ));

    let project = TempProject::new(&source_project).unwrap();

    run_update_with_specs(
        &project,
        vec![
            PackageSpec::Required(
                PackageReq::from_str("diamond-graph/direct-dependency-a@0.1.0".into()).unwrap(),
            ),
            PackageSpec::Named(PackageName::new("diamond-graph", "indirect-dependency-a").unwrap()),
        ],
    )
    .unwrap();

    let lockfile_content = fs::read_to_string(project.path().join("wally.lock")).unwrap();
    assert_snapshot!(lockfile_content);
}

fn run_update(project: &TempProject) -> anyhow::Result<()> {
    run_update_with_specs(project, Vec::new())
}

fn run_update_with_specs(project: &TempProject, specs: Vec<PackageSpec>) -> anyhow::Result<()> {
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
}
