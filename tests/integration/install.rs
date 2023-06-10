use std::path::{Path, PathBuf};

use fs_err as fs;
use libwally::{Args, GlobalOptions, InstallSubcommand, Subcommand};
use tempfile::{tempdir, TempDir};
use walkdir::WalkDir;

#[test]
fn minimal() {
    run_test("minimal");
}

#[test]
fn one_dependency() {
    run_test("one-dependency");
}

#[test]
fn transitive_dependency() {
    run_test("transitive-dependency");
}

#[test]
fn private_with_public_dependency() {
    run_test("private-with-public-dependency");
}

#[test]
fn dev_dependency() {
    run_test("dev-dependency");
}

#[test]
fn dev_dependency_also_required_as_non_dev() {
    run_test("dev-dependency-also-required-as-non-dev");
}

#[test]
fn cross_realm_dependency() {
    run_test("cross-realm-dependency");
}

#[test]
fn cross_realm_explicit_dependency() {
    run_test("cross-realm-explicit-dependency");
}

fn run_test(name: &str) -> TempProject {
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
        }),
    };

    args.run().unwrap();

    assert_dir_snapshot!(project.path());
    project
}

/// A handle to a project contained in a temporary directory. The project is
/// deleted when this type is dropped, which happens at the end of a test.
struct TempProject {
    dir: TempDir,
}

impl TempProject {
    fn new(source: &Path) -> anyhow::Result<Self> {
        let dir = tempdir()?;
        copy_dir_all(source, dir.path())?;

        Ok(Self { dir })
    }

    fn path(&self) -> &Path {
        self.dir.path()
    }

    /// Leak our reference to the temporary directory, allowing it to persist
    /// after the test runs. Useful for debugging.
    #[allow(unused)]
    fn leak(self) -> PathBuf {
        let path = self.dir.path().to_owned();
        let dir = Box::new(self.dir);
        Box::leak(dir);

        path
    }
}

/// Copy the contents of a directory into another directory. Because we use this
/// function with temp directories, the destination directory is expected to
/// already exist.
fn copy_dir_all(from: &Path, into: &Path) -> anyhow::Result<()> {
    let source = WalkDir::new(from).min_depth(1).follow_links(true);

    for entry in source {
        let entry = entry?;
        let relative_path = entry.path().strip_prefix(from).unwrap();
        let dest_path = into.join(relative_path);

        if entry.file_type().is_dir() {
            fs::create_dir(&dest_path)?;
        } else {
            fs::copy(entry.path(), dest_path)?;
        }
    }

    Ok(())
}
