use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::path::Path;
use std::process::{Command, Output};
use std::thread;
use std::time::{Duration, Instant};

use fs_err::File;
use libwally::{
    git_util, package_contents::PackageContents, package_index::PackageIndex,
    test_package::PackageBuilder, Args, GlobalOptions, PublishSubcommand, Subcommand,
};
use serial_test::serial;
use tempfile::tempdir;
use url::Url;

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
fn publish_unauthorized() {
    assert_publish_failure("401 Unauthorized", r#"{"message":"Invalid token"}"#);
}

#[test]
fn publish_validation_failure() {
    assert_publish_failure(
        "422 Unprocessable Entity",
        r#"{"message":"Version already exists"}"#,
    );
}

#[test]
fn publish_server_error() {
    assert_publish_failure("500 Internal Server Error", "Registry unavailable");
}

#[test]
fn publish_success() {
    for status in ["200 OK", "201 Created", "204 No Content"] {
        let output = publish_with_response(status, "");
        assert!(output.status.success(), "{:?}", output);
        assert!(output.stderr.is_empty(), "{:?}", output);
        assert!(String::from_utf8(output.stdout)
            .unwrap()
            .contains("Package published successfully!"));
    }
}

fn assert_publish_failure(status: &'static str, body: &'static str) {
    let output = publish_with_response(status, body);
    assert!(!output.status.success(), "{:?}", output);

    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains(status), "{}", stderr);
    assert!(stderr.contains(body), "{}", stderr);
    assert!(!stdout.contains(status), "{}", stdout);
    assert!(!stdout.contains(body), "{}", stdout);
    assert!(!stdout.contains("Package published successfully!"));
}

/// Run the real CLI against an isolated local registry to check its exit status
/// and output streams, without needing registry credentials or network access.
fn publish_with_response(status: &'static str, body: &'static str) -> Output {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let registry = tempdir().unwrap();
    let index_path = registry.path().join("index");
    fs_err::create_dir(&index_path).unwrap();
    fs_err::write(
        index_path.join("config.json"),
        serde_json::json!({ "api": format!("http://{}", listener.local_addr().unwrap()) })
            .to_string(),
    )
    .unwrap();
    git_util::init_test_repo(&index_path).unwrap();

    let index_url = Url::from_directory_path(&index_path).unwrap();
    let project = tempdir().unwrap();
    let mut manifest = PackageBuilder::new("test/publish@1.0.0").into_manifest();
    manifest.package.registry = index_url.to_string();
    fs_err::write(
        project.path().join("wally.toml"),
        toml::to_string(&manifest).unwrap(),
    )
    .unwrap();

    // The CLI caches indexes, so keep track of this test's unique cache entry
    // and remove it after the subprocess exits.
    let index = PackageIndex::new(&index_url, None).unwrap();
    let cache_path = index.path().clone();
    drop(index);

    let server = thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(30);
        let (mut stream, _) = loop {
            match listener.accept() {
                Ok(connection) => break connection,
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    assert!(Instant::now() < deadline, "Publish request timed out");
                    thread::sleep(Duration::from_millis(10));
                }
                Err(error) => panic!("Could not accept publish request: {}", error),
            }
        };
        stream
            .set_read_timeout(Some(Duration::from_secs(30)))
            .unwrap();
        let mut reader = BufReader::new(&mut stream);
        let mut line = String::new();
        reader.read_line(&mut line).unwrap();
        assert_eq!(line, "POST /v1/publish HTTP/1.1\r\n");
        let mut content_length = None;
        loop {
            line.clear();
            assert_ne!(reader.read_line(&mut line).unwrap(), 0);
            if line == "\r\n" {
                break;
            }
            if let Some((name, value)) = line.split_once(':') {
                if name.eq_ignore_ascii_case("content-length") {
                    content_length = Some(value.trim().parse::<usize>().unwrap());
                }
            }
        }
        let mut request_body = vec![0; content_length.unwrap()];
        reader.read_exact(&mut request_body).unwrap();
        stream
            .write_all(
                format!(
                    "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                )
                .as_bytes(),
            )
            .unwrap();
    });

    let output = Command::new(env!("CARGO_BIN_EXE_wally"))
        .arg("publish")
        .arg("--project-path")
        .arg(project.path())
        .args(["--token", "test-token"])
        .env("RUST_LOG", "off")
        .env("NO_PROXY", "127.0.0.1")
        .output();
    fs_err::remove_dir_all(cache_path).unwrap();
    server.join().unwrap();
    output.unwrap()
}
