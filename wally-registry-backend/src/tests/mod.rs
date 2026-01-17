use std::path::Path;

use figment::{providers::Serialized, Figment};
use libwally::test_package::PackageBuilder;
use rocket::{
    http::{Accept, ContentType, Header, Status},
    local::blocking::{Client, LocalResponse},
};

use crate::{auth::AuthMode, config::Config, server, storage::StorageMode};

fn init_test_index_remote() -> anyhow::Result<url::Url> {
    let temp_dir = tempfile::tempdir()?;
    let repo = git2::Repository::init_bare(temp_dir.path())?;
    let sig = git2::Signature::now("PackageUser", "PackageUser@localhost")?;
    let tree_id = repo.index()?.write_tree()?;
    let tree = repo.find_tree(tree_id)?;

    // Setting head is required so clones will clone main instead of master (which doesn't exist)
    repo.set_head("refs/heads/main")?;
    repo.commit(
        Some("refs/heads/main"),
        &sig,
        &sig,
        "Initial commit",
        &tree,
        &[],
    )?;

    Ok(url::Url::from_directory_path(temp_dir.keep()).unwrap())
}

fn add_test_packages(output_dir: &Path) -> anyhow::Result<()> {
    let packages_path = Path::new("src/tests/packages");

    for package in glob::glob("src/tests/packages/**/*.zip")?.filter_map(Result::ok) {
        let relative_path = package.strip_prefix(packages_path)?;
        let output = output_dir.join(relative_path);

        fs_err::create_dir_all(output.parent().unwrap())?;
        fs_err::copy(&package, output)?;
    }

    Ok(())
}

fn new_client(auth: AuthMode) -> Client {
    new_client_with_remote(auth, init_test_index_remote().unwrap())
}

fn new_client_with_remote(auth: AuthMode, index_url: url::Url) -> Client {
    let package_path = tempfile::tempdir().unwrap().keep();
    add_test_packages(&package_path).unwrap();

    let figment = Figment::from(rocket::Config::default()).merge(Serialized::globals(Config {
        index_url,
        storage: StorageMode::Local {
            path: Some(package_path),
        },
        auth,
        github_token: None,
        minimum_wally_version: None,
    }));

    Client::tracked(server(figment)).expect("valid rocket instance")
}

struct Expectation {
    status: Status,
    content_type: ContentType,
}

impl Expectation {
    fn assert(self, response: LocalResponse<'_>) {
        fn get_body(response: LocalResponse<'_>) -> String {
            let body = response
                .into_string()
                .unwrap_or_else(|| format!("<response body was not valid UTF-8>"));

            body
        }

        let status = response.status();

        if status != self.status {
            panic!(
                "Expected status {}, got {}\n{}",
                self.status,
                status,
                get_body(response)
            );
        }

        let content_type = response
            .content_type()
            .unwrap_or_else(|| panic!("response did not have a content type"));

        if content_type != self.content_type {
            panic!(
                "Expected content type {}, got {}\n{}",
                self.content_type,
                content_type,
                get_body(response),
            );
        }
    }
}

#[test]
fn root() {
    let client = new_client(AuthMode::Unauthenticated);
    let response = client.get("/").dispatch();

    Expectation {
        status: Status::Ok,
        content_type: ContentType::JSON,
    }
    .assert(response);
}

#[test]
fn read_minimal() {
    let client = new_client(AuthMode::Unauthenticated);
    let response = client
        .get("/v1/package-contents/biff/minimal/0.1.0")
        .dispatch();

    Expectation {
        status: Status::Ok,
        content_type: ContentType::GZIP,
    }
    .assert(response);
}

#[test]
fn read_404() {
    let client = new_client(AuthMode::Unauthenticated);
    let response = client
        .get("/v1/package-contents/biff/doesnt-exist/0.1.0")
        .dispatch();

    Expectation {
        status: Status::NotFound,
        content_type: ContentType::JSON,
    }
    .assert(response);
}

#[test]
fn publish_unauthenticated_401() {
    let client = new_client(AuthMode::Unauthenticated);
    let response = client.post("/v1/publish").header(Accept::JSON).dispatch();

    Expectation {
        status: Status::Unauthorized,
        content_type: ContentType::JSON,
    }
    .assert(response);
}

#[test]
fn publish() {
    let contents = PackageBuilder::new("biff/hello@1.0.0").contents();

    let client = new_client(AuthMode::ApiKey(String::from("hello")));
    let response = client
        .post("/v1/publish")
        .header(Accept::JSON)
        .body(contents.data())
        .header(Header::new("Authorization", "Bearer hello"))
        .dispatch();

    Expectation {
        status: Status::Ok,
        content_type: ContentType::JSON,
    }
    .assert(response);
}

#[test]
fn read_write_double_key() {
    let client = new_client(AuthMode::DoubleApiKey {
        read: None,
        write: String::from("A write key"),
    });

    // We can read with no API key
    let response = client
        .get("/v1/package-contents/biff/minimal/0.1.0")
        .dispatch();

    Expectation {
        status: Status::Ok,
        content_type: ContentType::GZIP,
    }
    .assert(response);

    // But we can't write with no API key
    let contents = PackageBuilder::new("biff/hello@1.0.0").contents();
    let response = client
        .post("/v1/publish")
        .header(Accept::JSON)
        .body(contents.data())
        .dispatch();

    Expectation {
        status: Status::Unauthorized,
        content_type: ContentType::JSON,
    }
    .assert(response);

    // Unless we use the correct API key
    let response = client
        .post("/v1/publish")
        .header(Accept::JSON)
        .body(contents.data())
        .header(Header::new("Authorization", "Bearer A write key"))
        .dispatch();

    Expectation {
        status: Status::Ok,
        content_type: ContentType::JSON,
    }
    .assert(response);
}

#[test]
fn publish_duplicate() {
    let contents = PackageBuilder::new("biff/hello@0.1.0").contents();
    let client = new_client(AuthMode::ApiKey(String::from("hello")));
    let send_request = || {
        client
            .post("/v1/publish")
            .header(Accept::JSON)
            .body(contents.data())
            .header(Header::new("Authorization", "Bearer hello"))
            .dispatch()
    };

    Expectation {
        status: Status::Ok,
        content_type: ContentType::JSON,
    }
    .assert(send_request());

    Expectation {
        status: Status::Conflict,
        content_type: ContentType::JSON,
    }
    .assert(send_request());
}

#[test]
fn publish_updates_git_remote() {
    let remote = init_test_index_remote().unwrap();
    let repo = git2::Repository::open(remote.to_file_path().unwrap()).unwrap();
    let client = new_client_with_remote(AuthMode::ApiKey(String::from("hello")), remote);

    let commit = repo.head().unwrap().peel_to_commit().unwrap();
    assert_eq!(commit.message().unwrap(), "Initial commit");

    let contents = PackageBuilder::new("biff/hello@0.1.0").contents();
    client
        .post("/v1/publish")
        .header(Accept::JSON)
        .body(contents.data())
        .header(Header::new("Authorization", "Bearer hello"))
        .dispatch();

    let commit = repo.head().unwrap().peel_to_commit().unwrap();
    assert_ne!(commit.message().unwrap(), "Initial commit");
}

/// Ensures the package index will update when the remote is modified before a publish request
/// This is to check our update implementation as well as support for multiple api nodes
#[test]
fn publish_then_publish_elsewhere() {
    let remote = init_test_index_remote().unwrap();
    let client1 = new_client_with_remote(AuthMode::ApiKey(String::from("hello")), remote.clone());
    let client2 = new_client_with_remote(AuthMode::ApiKey(String::from("hello")), remote);

    let contents = PackageBuilder::new("biff/hello@1.0.0").contents();
    let response = client1
        .post("/v1/publish")
        .header(Accept::JSON)
        .body(contents.data())
        .header(Header::new("Authorization", "Bearer hello"))
        .dispatch();

    Expectation {
        status: Status::Ok,
        content_type: ContentType::JSON,
    }
    .assert(response);

    let contents = PackageBuilder::new("biff/hello@1.0.1").contents();
    let response = client2
        .post("/v1/publish")
        .header(Accept::JSON)
        .body(contents.data())
        .header(Header::new("Authorization", "Bearer hello"))
        .dispatch();

    Expectation {
        status: Status::Ok,
        content_type: ContentType::JSON,
    }
    .assert(response);
}

// TODO: Implement yanking
#[test]
#[ignore]
fn yank() {}
