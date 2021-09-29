use std::path::Path;

use fs_err::File;
use libwally::{
    package_contents::PackageContents, Args, GlobalOptions, PublishSubcommand, Subcommand,
};
use tempfile::tempdir;

/// If the user tries to publish without providing any auth tokens
/// then we should prompt them to provide a token via 'wally login'
#[test]
fn check_prompts_auth() {
    let test_projects = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/test-projects",));

    let args = Args {
        global: GlobalOptions {
            use_temp_index: true,
            ..Default::default()
        },
        subcommand: Subcommand::Publish(PublishSubcommand {
            project_path: test_projects.join("minimal"),
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
    let test_projects = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/test-projects",));
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
