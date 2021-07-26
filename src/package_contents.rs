use std::io::{self, BufReader, Cursor};
use std::path::Path;

use anyhow::format_err;
use fs_err::File;
use walkdir::WalkDir;
use zip::{write::FileOptions, ZipArchive, ZipWriter};

static EXCLUDED_PATHS: &[&str] = &[".git", "wally.lock"];

/// Container for the contents of a package that have been downloaded.
#[derive(Clone)]
pub struct PackageContents {
    /// Contains a zip with the contents of the package.
    data: Vec<u8>,
}

impl PackageContents {
    pub fn pack_from_path(input: &Path) -> anyhow::Result<Self> {
        let mut data = Vec::new();
        let mut archive = ZipWriter::new(Cursor::new(&mut data));

        let file_iterator = WalkDir::new(input)
            .min_depth(1)
            .into_iter()
            .filter_entry(|entry| dir_entry_filter(entry.path().strip_prefix(input).unwrap()));

        for entry in file_iterator {
            let entry = entry?;

            let path = entry.path();
            let relative_path = path.strip_prefix(input).unwrap();
            let archive_name = relative_path.to_str().ok_or_else(|| {
                format_err!(
                    "Path {} contained invalid Unicode characters",
                    relative_path.display()
                )
            })?;

            if entry.file_type().is_dir() {
                archive.add_directory(archive_name, FileOptions::default())?;
            } else {
                archive.start_file(archive_name, FileOptions::default())?;
                let mut file = BufReader::new(File::open(path)?);
                io::copy(&mut file, &mut archive)?;
            }
        }

        archive.finish()?;
        drop(archive);

        Ok(PackageContents { data })
    }

    /// Unpack the package into the given path on the filesystem.
    pub fn unpack_into_path(&self, output: &Path) -> anyhow::Result<()> {
        let mut archive = ZipArchive::new(Cursor::new(self.data.as_slice()))?;
        archive.extract(output)?;
        Ok(())
    }

    pub fn data(&self) -> &[u8] {
        &self.data
    }

    /// Create a new PackageContents object from a buffer.
    pub fn from_buffer(data: Vec<u8>) -> PackageContents {
        PackageContents { data }
    }
}

fn dir_entry_filter(path: &Path) -> bool {
    !EXCLUDED_PATHS.iter().any(|p| Path::new(p) == path)
}
