use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::path::Path;
use std::thread;

use fs_err::File;
use libwally::{
    Args, GlobalOptions, PublishSubcommand, Subcommand, git_util, package_contents::PackageContents,
};
use serial_test::serial;
use tempfile::tempdir;

use crate::temp_project::TempProject;

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

#[test]
fn check_rejected_publish_errors() {
    let error = publish_with_response("422 Unprocessable Entity", "Version already exists")
        .expect_err("Expected publish to return an error");

    let message = error.to_string();
    assert!(message.contains("422"), "{}", message);
    assert!(message.contains("Version already exists"), "{}", message);
}

#[test]
fn check_accepted_publish_succeeds() {
    publish_with_response("200 OK", "").expect("Expected publish to succeed");
}

fn publish_with_response(status: &str, body: &str) -> anyhow::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let api = format!("http://{}", listener.local_addr().unwrap());

    let registry = tempdir().unwrap();
    let index_path = registry.path().join("index");
    fs_err::create_dir(&index_path).unwrap();
    fs_err::write(
        index_path.join("config.json"),
        format!(r#"{{"api":"{api}"}}"#),
    )
    .unwrap();
    git_util::init_test_repo(&index_path).unwrap();

    let project = TempProject::new(Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/test-projects/minimal"
    )))
    .unwrap();
    let manifest_path = project.path().join("wally.toml");
    let manifest = fs_err::read_to_string(&manifest_path).unwrap().replace(
        r#""test-registries/primary-registry""#,
        &format!("'{}'", registry.path().display()),
    );
    fs_err::write(&manifest_path, manifest).unwrap();

    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    thread::spawn(move || {
        let (stream, _) = listener.accept().unwrap();
        let mut reader = BufReader::new(stream);

        let mut content_length = 0;
        let mut line = String::new();
        while reader.read_line(&mut line).unwrap() > 2 {
            if let Some(value) = line.to_ascii_lowercase().strip_prefix("content-length:") {
                content_length = value.trim().parse().unwrap();
            }
            line.clear();
        }
        reader.read_exact(&mut vec![0; content_length]).unwrap();

        reader.into_inner().write_all(response.as_bytes()).unwrap();
    });

    Args {
        global: GlobalOptions {
            test_registry: true,
            use_temp_index: true,
            ..Default::default()
        },
        subcommand: Subcommand::Publish(PublishSubcommand {
            project_path: project.path().to_owned(),
            token: Some("token".to_owned()),
        }),
    }
    .run()
}
