use std::path::Path;

use libwally::{Args, GlobalOptions, PublishSubcommand, Subcommand};

/// If the user tries to publish without providing any auth tokens
/// then we should prompt them to provide a token via 'wally login'
#[test]
fn check_prompts_auth() {
    let test_projects = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/test-projects",));

    let args = Args {
        global: GlobalOptions::default(),
        subcommand: Subcommand::Publish(PublishSubcommand {
            project_path: test_projects.join("minimal"),
        }),
    };

    let error = args
        .run()
        .expect_err("Expected publish to return an error")
        .to_string();

    assert!(
        error.contains("wally login"),
        "Expected error message prompting user to login. Instead we got: {}",
        error
    )
}

/// If the names in wally.toml and default.project.json are mismatched then
/// publish should fail with an explanatory error message
#[test]
fn check_mismatched_names() {
    let test_projects = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/test-projects",));

    let args = Args {
        global: GlobalOptions {
            use_temp_index: true,
            ..Default::default()
        },
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
        "Expected error message explaining mismatched package name. Instead we got: {}",
        error
    )
}
