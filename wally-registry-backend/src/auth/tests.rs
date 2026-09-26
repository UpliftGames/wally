use super::*;
use libwally::package_id::PackageId;
use std::str::FromStr;
use tempfile::tempdir;

fn create_test_index() -> PackageIndex {
    let temp_dir = tempdir().unwrap();
    let repo = git2::Repository::init_bare(temp_dir.path()).unwrap();
    let sig = git2::Signature::now("Test", "test@test.com").unwrap();
    let tree_id = repo.index().unwrap().write_tree().unwrap();
    let tree = repo.find_tree(tree_id).unwrap();
    repo.set_head("refs/heads/main").unwrap();
    repo.commit(Some("refs/heads/main"), &sig, &sig, "Initial", &tree, &[])
        .unwrap();

    let url = url::Url::from_directory_path(temp_dir.into_path()).unwrap();
    PackageIndex::new(&url, None).unwrap()
}

fn github_info(login: &str, id: u64, orgs: Vec<&str>) -> GithubInfo {
    GithubInfo {
        login: login.to_string(),
        id,
        orgs: orgs.into_iter().map(|s| s.to_string()).collect(),
    }
}

#[test]
fn user_can_publish_to_own_scope() {
    let index = create_test_index();
    let package_id = PackageId::from_str("gmackie/mypackage@1.0.0").unwrap();
    let write_access = WriteAccess::Github(github_info("gmackie", 123, vec![]));

    assert!(write_access.can_write_package(&package_id, &index).unwrap());
}

#[test]
fn user_cannot_publish_to_other_user_scope() {
    let index = create_test_index();
    let package_id = PackageId::from_str("otheruser/mypackage@1.0.0").unwrap();
    let write_access = WriteAccess::Github(github_info("gmackie", 123, vec![]));

    assert!(!write_access.can_write_package(&package_id, &index).unwrap());
}

#[test]
fn user_can_publish_to_org_scope_as_member() {
    let index = create_test_index();
    let package_id = PackageId::from_str("gmackorg/mypackage@1.0.0").unwrap();
    let write_access = WriteAccess::Github(github_info("gmackie", 123, vec!["gmackorg"]));

    assert!(write_access.can_write_package(&package_id, &index).unwrap());
}

#[test]
fn user_cannot_publish_to_org_scope_as_non_member() {
    let index = create_test_index();
    let package_id = PackageId::from_str("someorg/mypackage@1.0.0").unwrap();
    let write_access = WriteAccess::Github(github_info("gmackie", 123, vec!["differentorg"]));

    assert!(!write_access.can_write_package(&package_id, &index).unwrap());
}

#[test]
fn orgs_must_be_lowercase_to_match() {
    let index = create_test_index();
    let package_id = PackageId::from_str("gmackorg/mypackage@1.0.0").unwrap();
    let write_access = WriteAccess::Github(github_info("gmackie", 123, vec!["GmackOrg"]));

    assert!(!write_access.can_write_package(&package_id, &index).unwrap());
}

#[test]
fn api_key_can_publish_anywhere() {
    let index = create_test_index();
    let package_id = PackageId::from_str("anyuser/anypackage@1.0.0").unwrap();
    let write_access = WriteAccess::ApiKey;

    assert!(write_access.can_write_package(&package_id, &index).unwrap());
}
