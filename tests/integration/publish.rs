use std::path::Path;

use fs_err::File;
use libwally::{
    git_util, manifest::Manifest, package_contents::PackageContents, package_name::PackageName,
    workspace::Workspace, Args, GlobalOptions, PublishSubcommand, Subcommand,
};
use serial_test::serial;
use tempfile::tempdir;

use super::temp_project::TempProject;

/// If the user tries to publish without providing any auth tokens
/// then we should prompt them to provide a token via 'wally login'
#[test]
#[serial]
fn check_prompts_auth() {
    let test_projects = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/test-projects"));
    let test_registry = Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/test-registries/primary-registry"
    ));

    git_util::init_test_repo(&test_registry.join("index")).unwrap();

    let args = Args {
        global: GlobalOptions {
            test_registry: true,
            use_temp_index: true,
            ..Default::default()
        },
        subcommand: Subcommand::Publish(PublishSubcommand {
            project_path: test_projects.join("minimal"),
            token: None,
            package: None,
        }),
    };

    let error = args.run().expect_err("Expected publish to return an error");

    assert!(
        error.to_string().contains("wally login"),
        "Expected error message prompting user to login. Instead we got: {:#}",
        error
    )
}

/// If the names in wally.toml and default.project.json are mismatched then
/// publish should edit the default.project.json during upload to match
#[test]
fn check_mismatched_names() {
    let test_projects = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/test-projects"));
    let contents = PackageContents::pack_from_path(&test_projects.join("mismatched-name")).unwrap();

    let unpacked_contents = tempdir().unwrap();
    contents.unpack_into_path(unpacked_contents.path()).unwrap();

    let project_json_path = unpacked_contents.path().join("default.project.json");

    let file = File::open(project_json_path).unwrap();
    let project_json: serde_json::Value = serde_json::from_reader(file).unwrap();
    let project_name = project_json
        .get("name")
        .and_then(|name| name.as_str())
        .expect("Couldn't parse name in default.project.json");

    // default.project.json should now contain mismatched-name instead of Mismatched-name
    assert_eq!(project_name, "mismatched-name");
}

/// If the private field in wally.toml is set to true, it should not publish
/// the package.
#[test]
fn check_private_field() {
    let test_projects = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/test-projects"));

    let args = Args {
        global: GlobalOptions {
            test_registry: true,
            use_temp_index: true,
            ..Default::default()
        },
        subcommand: Subcommand::Publish(PublishSubcommand {
            project_path: test_projects.join("private-package"),
            token: None,
            package: None,
        }),
    };

    let error = args.run().expect_err("Expected publish to return an error");

    assert!(
        error.to_string().contains("Cannot publish"),
        "Expected error message that a private package cannot be published. Instead we got: {:#}",
        error
    )
}

/// Ensure a token passed as an optional argument is correctly used in the request
#[test]
#[serial]
fn check_token_arg() {
    let test_projects = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/test-projects"));
    let test_registry = Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/test-registries/primary-registry"
    ));

    git_util::init_test_repo(&test_registry.join("index")).unwrap();

    let args = Args {
        global: GlobalOptions {
            test_registry: true,
            use_temp_index: true,
            check_token: Some("token".to_owned()),
            ..Default::default()
        },
        subcommand: Subcommand::Publish(PublishSubcommand {
            project_path: test_projects.join("minimal"),
            token: Some("token".to_owned()),
            package: None,
        }),
    };

    args.run()
        .expect("Publish did not use the provided token in the publish request");
}

/// Workspace publish: path deps are rewritten to registry deps in the packed manifest.
/// The packed manifest should be a standard Wally manifest with no path deps.
#[test]
fn workspace_publish_rewrites_path_deps() {
    let source_project = Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/test-projects/workspace-publish"
    ));
    let project = TempProject::new(source_project).unwrap();

    let workspace = Workspace::load(project.path()).unwrap();
    let pkg_name: PackageName = "scope/publishable".parse().unwrap();
    let (member_dir, manifest) = workspace.find_member_by_name(&pkg_name).unwrap();

    assert!(
        manifest.dependencies.get("Sibling").unwrap().as_path().is_some(),
        "fixture should have a path dep for Sibling"
    );

    // Simulate what publish does: rewrite path deps and pack
    let rewritten = rewrite_for_publish(manifest, member_dir, &workspace);

    // The rewritten manifest should have no path deps
    for (alias, spec) in &rewritten.dependencies {
        assert!(
            spec.as_registry().is_some(),
            "dependency '{}' should be rewritten to registry, got: {:?}",
            alias,
            spec
        );
    }

    let sibling_dep = rewritten.dependencies.get("Sibling").unwrap();
    let sibling_req = sibling_dep.as_registry().unwrap();
    assert_eq!(sibling_req.name().to_string(), "scope/sibling");

    // Create temp copy with rewritten manifest and pack it
    let temp_dir = tempdir().unwrap();
    copy_dir_contents(member_dir, temp_dir.path());
    let rewritten_toml = toml::to_string_pretty(&rewritten).unwrap();
    fs_err::write(temp_dir.path().join("wally.toml"), rewritten_toml).unwrap();
    let contents = PackageContents::pack_from_path(temp_dir.path()).unwrap();

    // Unpack and verify the packed manifest is a valid standard manifest
    let unpacked = tempdir().unwrap();
    contents.unpack_into_path(unpacked.path()).unwrap();
    let packed_manifest = Manifest::load(unpacked.path()).unwrap();
    assert!(
        packed_manifest.dependencies.get("Sibling").unwrap().as_registry().is_some(),
        "packed manifest should have registry dep for Sibling"
    );
}

/// Workspace publish: --package selects the correct member
#[test]
#[serial]
fn workspace_publish_package_selects_member() {
    let source_project = Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/test-projects/workspace-publish"
    ));
    let test_registry = Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/test-registries/primary-registry"
    ));

    git_util::init_test_repo(&test_registry.join("index")).unwrap();
    let project = TempProject::new(source_project).unwrap();

    let args = Args {
        global: GlobalOptions {
            test_registry: true,
            use_temp_index: true,
            check_token: Some("test-token".to_owned()),
            ..Default::default()
        },
        subcommand: Subcommand::Publish(PublishSubcommand {
            project_path: project.path().to_owned(),
            token: Some("test-token".to_owned()),
            package: Some("scope/sibling".parse().unwrap()),
        }),
    };

    args.run().expect("workspace publish of scope/sibling should succeed");
}

/// Workspace publish: error when publishing member that depends on private member
#[test]
fn workspace_publish_errors_on_private_dep() {
    let source_project = Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/test-projects/workspace-publish"
    ));

    // Add a path dep from publishable to private-member in a temp copy
    let project = TempProject::new(source_project).unwrap();

    // Modify publishable's manifest to also depend on the private member
    let publishable_manifest = project.path().join("modules/publishable/wally.toml");
    let manifest_content = fs_err::read_to_string(&publishable_manifest).unwrap();
    let new_content = manifest_content.replace(
        "[dependencies]\nSibling = { path = \"../sibling\" }",
        "[dependencies]\nSibling = { path = \"../sibling\" }\nPrivate = { path = \"../private-member\" }",
    );
    fs_err::write(&publishable_manifest, new_content).unwrap();

    let workspace = Workspace::load(project.path()).unwrap();
    let pkg_name: PackageName = "scope/publishable".parse().unwrap();
    let (member_dir, manifest) = workspace.find_member_by_name(&pkg_name).unwrap();

    let result = validate_publish_graph_test(manifest, member_dir, &workspace);
    assert!(result.is_err(), "should error on private dep");
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("private"),
        "error should mention private: {}",
        err
    );
}

/// Workspace publish: --package without workspace is an error
#[test]
fn workspace_publish_package_flag_without_workspace_errors() {
    let test_projects = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/test-projects"));

    let args = Args {
        global: GlobalOptions {
            test_registry: true,
            use_temp_index: true,
            ..Default::default()
        },
        subcommand: Subcommand::Publish(PublishSubcommand {
            project_path: test_projects.join("minimal"),
            token: None,
            package: Some("biff/minimal".parse().unwrap()),
        }),
    };

    let error = args.run().expect_err("Expected error for --package without workspace");
    assert!(
        error.to_string().contains("single-package"),
        "Error should mention single-package: {:#}",
        error
    );
}

/// Workspace publish: publishing without --package in a workspace errors
#[test]
fn workspace_publish_requires_package_flag() {
    let source_project = Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/test-projects/workspace-publish"
    ));
    let project = TempProject::new(source_project).unwrap();

    let args = Args {
        global: GlobalOptions {
            test_registry: true,
            use_temp_index: true,
            ..Default::default()
        },
        subcommand: Subcommand::Publish(PublishSubcommand {
            project_path: project.path().to_owned(),
            token: None,
            package: None,
        }),
    };

    let error = args.run().expect_err("Expected error when no --package specified");
    assert!(
        error.to_string().contains("--package"),
        "Error should mention --package: {:#}",
        error
    );
}

// Helper functions used by workspace publish tests

fn rewrite_for_publish(
    manifest: &Manifest,
    member_dir: &Path,
    workspace: &Workspace,
) -> Manifest {
    use libwally::dependency_spec::DependencySpec;
    use libwally::package_req::PackageReq;
    use semver::VersionReq;

    let mut rewritten = manifest.clone();

    fn rewrite_deps(
        deps: &mut std::collections::BTreeMap<String, DependencySpec>,
        member_dir: &Path,
        workspace: &Workspace,
    ) {
        for (_alias, spec) in deps.iter_mut() {
            if let DependencySpec::Path(path_spec) = spec {
                let target_dir = member_dir.join(&path_spec.path).canonicalize().unwrap();
                let target = workspace.get_member_at_path(&target_dir).unwrap();
                let exact_req = VersionReq::exact(&target.package.version);
                *spec = DependencySpec::Registry(PackageReq::new(
                    target.package.name.clone(),
                    exact_req,
                ));
            }
        }
    }

    rewrite_deps(&mut rewritten.dependencies, member_dir, workspace);
    rewrite_deps(&mut rewritten.server_dependencies, member_dir, workspace);
    rewrite_deps(&mut rewritten.dev_dependencies, member_dir, workspace);
    rewritten
}

fn validate_publish_graph_test(
    manifest: &Manifest,
    member_dir: &Path,
    workspace: &Workspace,
) -> anyhow::Result<()> {
    use libwally::dependency_spec::DependencySpec;

    let all_deps = manifest
        .dependencies
        .iter()
        .chain(manifest.server_dependencies.iter())
        .chain(manifest.dev_dependencies.iter());

    for (alias, spec) in all_deps {
        if let DependencySpec::Path(path_spec) = spec {
            let target_dir = member_dir.join(&path_spec.path);
            if let Ok(target_dir) = target_dir.canonicalize() {
                if let Some(target) = workspace.get_member_at_path(&target_dir) {
                    if target.package.private {
                        anyhow::bail!(
                            "cannot publish '{}': it depends on private member '{}' (via '{}')",
                            manifest.package.name,
                            target.package.name,
                            alias,
                        );
                    }
                }
            }
        }
    }
    Ok(())
}

fn copy_dir_contents(from: &Path, into: &Path) {
    use walkdir::WalkDir;
    for entry in WalkDir::new(from).min_depth(1).follow_links(true) {
        let entry = entry.unwrap();
        let relative_path = entry.path().strip_prefix(from).unwrap();
        let dest_path = into.join(relative_path);
        if entry.file_type().is_dir() {
            fs_err::create_dir(&dest_path).unwrap();
        } else {
            fs_err::copy(entry.path(), dest_path).unwrap();
        }
    }
}
