use fs_err as fs;
use std::path::{Path, PathBuf};
use tempfile::{tempdir, TempDir};
use walkdir::WalkDir;

/// A handle to a project contained in a temporary directory. The project is
/// deleted when this type is dropped, which happens at the end of a test.
pub struct TempProject {
    dir: TempDir,
}

impl TempProject {
    pub fn new(source: &Path) -> anyhow::Result<Self> {
        let dir = tempdir()?;
        copy_dir_all(source, dir.path())?;

        Ok(Self { dir })
    }

    pub fn path(&self) -> &Path {
        self.dir.path()
    }

    /// Leak our reference to the temporary directory, allowing it to persist
    /// after the test runs. Useful for debugging.
    #[allow(unused)]
    pub fn leak(self) -> PathBuf {
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
