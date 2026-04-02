use std::collections::BTreeSet;
use std::path::Path;

use libwally::{
    installation::InstallationContext,
    lockfile::Lockfile,
    package_id::PackageId,
    package_source::{PackageSource, PackageSourceMap, TestRegistry},
    resolution::resolve_workspace,
    workspace::Workspace,
    Args, GlobalOptions, InstallSubcommand, Subcommand,
};

use super::temp_project::TempProject;

fn run_workspace_install_test(name: &str) -> TempProject {
    let source_project =
        Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/test-projects")).join(name);
    let project = TempProject::new(&source_project).unwrap();

    let workspace = Workspace::load(project.path()).unwrap();

    let registry_path = Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/test-registries/primary-registry"
    ));
    let default_registry: Box<PackageSource> =
        Box::new(PackageSource::TestRegistry(TestRegistry::new(registry_path)));
    let mut sources = PackageSourceMap::new(default_registry);
    sources.add_fallbacks().unwrap();

    let resolved = resolve_workspace(&workspace, &Default::default(), &sources).unwrap();

    Lockfile::from_resolve(&resolved, Some(workspace.root()))
        .save(project.path())
        .unwrap();

    let root_ids: BTreeSet<PackageId> = workspace
        .members()
        .values()
        .map(|m| m.package_id())
        .collect();

    let ctx = InstallationContext::new(
        project.path(),
        workspace.place().shared_packages.clone(),
        workspace.place().server_packages.clone(),
    );
    ctx.clean().unwrap();
    ctx.install(sources, root_ids, resolved).unwrap();

    assert_dir_snapshot!(project.path());
    project
}

/// Run workspace install via the CLI `Args` path, exercising workspace root
/// discovery and the full install command.
fn run_workspace_install_cli(name: &str) -> TempProject {
    let source_project =
        Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/test-projects")).join(name);
    let project = TempProject::new(&source_project).unwrap();

    Args {
        global: GlobalOptions {
            test_registry: true,
            ..Default::default()
        },
        subcommand: Subcommand::Install(InstallSubcommand {
            project_path: project.path().to_owned(),
            locked: false,
        }),
    }
    .run()
    .unwrap();

    project
}

#[test]
fn workspace_virtual() {
    run_workspace_install_test("workspace-virtual");
}

#[test]
fn workspace_path_dep() {
    run_workspace_install_test("workspace-path-dep");
}

#[test]
fn workspace_with_registry_dep() {
    run_workspace_install_test("workspace-with-registry-dep");
}

#[test]
fn workspace_inheritance() {
    run_workspace_install_test("workspace-inheritance");
}

#[test]
fn workspace_hybrid_root() {
    run_workspace_install_test("workspace-hybrid-root");
}

#[test]
fn workspace_cross_realm() {
    run_workspace_install_test("workspace-cross-realm");
}

// =========================================================================
// CLI-path tests (exercise the full Args::run() flow)
// =========================================================================

/// End-to-end: workspace install via CLI for virtual workspace.
#[test]
fn workspace_virtual_cli() {
    let project = run_workspace_install_cli("workspace-virtual");
    // Verify lockfile was created at workspace root
    assert!(project.path().join("wally.lock").exists());
    // Verify packages directory was created
    assert!(project.path().join("ServerPackages").exists());
}

/// End-to-end: workspace install via CLI for workspace with path deps.
#[test]
fn workspace_path_dep_cli() {
    let project = run_workspace_install_cli("workspace-path-dep");
    assert!(project.path().join("wally.lock").exists());
    assert!(project.path().join("Packages").exists());
}

/// End-to-end: workspace install via CLI for workspace with registry deps.
#[test]
fn workspace_with_registry_dep_cli() {
    let project = run_workspace_install_cli("workspace-with-registry-dep");
    assert!(project.path().join("wally.lock").exists());
}

/// Workspace root discovery from a member subdirectory.
#[test]
fn workspace_root_discovery_from_member() {
    let source_project = Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/test-projects/workspace-path-dep"
    ));
    let project = TempProject::new(source_project).unwrap();

    // Install from a member subdirectory -- discovery should walk up to find the workspace root
    let member_dir = project.path().join("modules/foo");
    Args {
        global: GlobalOptions {
            test_registry: true,
            ..Default::default()
        },
        subcommand: Subcommand::Install(InstallSubcommand {
            project_path: member_dir.clone(),
            locked: false,
        }),
    }
    .run()
    .unwrap();

    // The lockfile and Packages should be at the workspace root, not in the member dir
    assert!(project.path().join("wally.lock").exists());
    assert!(
        !member_dir.join("wally.lock").exists(),
        "lockfile should be at workspace root, not in member dir"
    );
}

/// Single-package projects still work through the workspace-aware install.
#[test]
fn single_package_backward_compat_cli() {
    let source_project = Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/test-projects/one-dependency"
    ));
    let project = TempProject::new(source_project).unwrap();

    Args {
        global: GlobalOptions {
            test_registry: true,
            ..Default::default()
        },
        subcommand: Subcommand::Install(InstallSubcommand {
            project_path: project.path().to_owned(),
            locked: false,
        }),
    }
    .run()
    .unwrap();

    assert!(project.path().join("wally.lock").exists());
    // one-dependency uses server realm, so packages go to ServerPackages/
    assert!(project.path().join("ServerPackages").exists());
}
