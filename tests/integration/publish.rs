use std::path::Path;

use fs_err::File;
use libwally::{
    git_util, package_contents::PackageContents, Args, GlobalOptions, PublishSubcommand, Subcommand,
};
use serial_test::serial;
use tempfile::tempdir;

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
        }),
    };

    args.run()
        .expect("Publish did not use the provided token in the publish request");
}
