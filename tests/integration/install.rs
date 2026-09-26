use super::temp_project::TempProject;
use libwally::{Args, GlobalOptions, InstallSubcommand, Subcommand};
use std::path::Path;

#[test]
fn minimal() {
    run_install_test("minimal");
}

#[test]
fn one_dependency() {
    run_install_test("one-dependency");
}

#[test]
fn transitive_dependency() {
    run_install_test("transitive-dependency");
}

#[test]
fn private_with_public_dependency() {
    run_install_test("private-with-public-dependency");
}

#[test]
fn dev_dependency() {
    run_install_test("dev-dependency");
}

#[test]
fn dev_dependency_also_required_as_non_dev() {
    run_install_test("dev-dependency-also-required-as-non-dev");
}

#[test]
fn cross_realm_dependency() {
    run_install_test("cross-realm-dependency");
}

#[test]
fn cross_realm_explicit_dependency() {
    run_install_test("cross-realm-explicit-dependency");
}

#[test]
fn manifest_links() {
    run_install_test("manifest-links");
}

#[test]
fn locked_pass() {
    let result = run_locked_install("diamond-graph/root/latest");

    assert!(result.is_ok(), "Should pass without any problems");
}

#[test]
fn locked_catches_dated_packages() {
    let result = run_locked_install("diamond-graph/root/dated");
    assert!(result.is_err(), "Should fail!");
}

fn run_locked_install(name: &str) -> Result<(), anyhow::Error> {
    let source_project =
        Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/test-projects",)).join(name);

    let project = TempProject::new(&source_project).unwrap();

    Args {
        global: GlobalOptions {
            test_registry: true,
            ..Default::default()
        },
        subcommand: Subcommand::Install(InstallSubcommand {
            project_path: project.path().to_owned(),
            locked: true,
        }),
    }
    .run()
}

fn run_install_test(name: &str) -> TempProject {
    let source_project =
        Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/test-projects",)).join(name);

    let project = TempProject::new(&source_project).unwrap();

    let args = Args {
        global: GlobalOptions {
            test_registry: true,
            ..Default::default()
        },
        subcommand: Subcommand::Install(InstallSubcommand {
            project_path: project.path().to_owned(),
            locked: false,
        }),
    };

    args.run().unwrap();

    assert_dir_snapshot!(project.path());
    project
}


#[test]
fn type_reexports_variadics_and_defaults() {
    use fs_err as fs;
    use tempfile::tempdir;
    use libwally::{Args, GlobalOptions, InstallSubcommand, Subcommand};
    use libwally::test_package::PackageBuilder;

    // 1) Create a temporary test registry on disk
    let reg_dir = tempdir().unwrap();
    let reg_path = reg_dir.path();

    fs::create_dir_all(reg_path.join("index").join("test")).unwrap();
    fs::create_dir_all(reg_path.join("contents").join("test").join("typed-exports")).unwrap();
    // Minimal config for TestRegistry fallback
    fs::write(
        reg_path.join("index").join("config.json"),
        r#"{"api":"http://localhost","fallback_registries":[]}"#,
    )
    .unwrap();

    // 2) Build a package with exported types (including defaults and variadic packs)
    let pkg = PackageBuilder::new("test/typed-exports@0.1.0").with_file(
        "src/init.luau",
        r#"export type Foo = number
export type Bar<T, S = unknown> = { x: T, y: S }
export type Variadic<T...> = (T...)
return {}
"#,
    );

    // Add manifest entry to registry index (as JSON lines)
    let mut manifest = pkg.manifest().clone();
    manifest.package.registry = reg_path.to_str().unwrap().replace('\\', "/");
    let index_path = reg_path.join("index").join("test").join("typed-exports");
    fs::write(
        &index_path,
        format!("{}\n", serde_json::to_string(&manifest).unwrap()),
    )
    .unwrap();

    // Add contents zip to registry contents/
    let contents = pkg.contents();
    let contents_path = reg_path
        .join("contents")
        .join("test")
        .join("typed-exports")
        .join("0.1.0.zip");
    fs::write(contents_path, contents.data()).unwrap();

    // 3) Create a temporary project that depends on our typed package
    let proj_dir = tempdir().unwrap();
    let project_path = proj_dir.path();

    fs::create_dir_all(project_path.join("src")).unwrap();
    fs::write(project_path.join("src").join("init.lua"), "return \"ok\"\n").unwrap();

    // Write wally.toml pointing registry to our temp registry path
    let wally_toml = format!(
        r#"[package]
name = "example/app"
version = "0.1.0"
license = "MIT"
realm = "shared"
registry = "{registry}"

[dependencies]
Typed = "test/typed-exports@0.1.0"
"#,
        registry = manifest.package.registry
    );
    fs::write(project_path.join("wally.toml"), wally_toml).unwrap();

    // Optional default.project.json
    fs::write(
        project_path.join("default.project.json"),
        "{\n  \"name\": \"app\",\n  \"tree\": {\"$path\": \"src\"}\n}\n",
    )
    .unwrap();

    // 4) Run `wally install` against our temp registry
    let args = Args {
        global: GlobalOptions {
            test_registry: true,
            ..Default::default()
        },
        subcommand: Subcommand::Install(InstallSubcommand {
            project_path: project_path.to_owned(),
            locked: false,
        }),
    };
    args.run().unwrap();

    // 5) Verify the generated thunk contains type re-exports
    let link = fs::read_to_string(project_path.join("Packages").join("Typed.lua")).unwrap();

    assert!(
        link.contains("local REQUIRED_MODULE = require("),
        "expected thunk to define REQUIRED_MODULE, got:\n{}",
        link
    );
    assert!(
        link.contains("export type Foo = REQUIRED_MODULE.Foo"),
        "expected Foo re-export, got:\n{}",
        link
    );
    assert!(
        link.contains("export type Bar<T, S = unknown> = REQUIRED_MODULE.Bar<T, S>"),
        "expected Bar re-export with defaults kept on LHS, got:\n{}",
        link
    );
    assert!(
        link.contains("export type Variadic<T...> = REQUIRED_MODULE.Variadic<T...>"),
        "expected variadic generic pack re-export, got:\n{}",
        link
    );
    assert!(
        link.ends_with("return REQUIRED_MODULE\n")
            || link.ends_with("return REQUIRED_MODULE\r\n"),
        "expected to return REQUIRED_MODULE, got:\n{}",
        link
    );
}
