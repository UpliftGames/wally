use std::path::Path;

use libwally::{Args, GlobalOptions, PublishSubcommand, Subcommand};

/// If the names in wally.toml and default.project.json are mismatched then
/// publish should fail with an explanatory error message
#[test]
fn check_mismatched_named() {
    let test_projects = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/test-projects",));

    let args = Args {
        global: GlobalOptions::default(),
        subcommand: Subcommand::Publish(PublishSubcommand {
            project_path: test_projects.join("mismatched-name"),
        }),
    };

    let error = args
        .run()
        .expect_err("Expected publish to return an error")
        .to_string();

    assert!(
        error.contains("mismatched"),
        "Expected error message explaining mismatched package name"
    )
}
