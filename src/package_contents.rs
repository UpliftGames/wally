use std::io::{self, BufRead, BufReader, Cursor};
use std::path::{Path, PathBuf};

use anyhow::format_err;
use fs_err::File;
use globset::{Glob, GlobSet, GlobSetBuilder};
use serde_json::json;
use walkdir::WalkDir;
use zip::{write::FileOptions, ZipArchive, ZipWriter};

use crate::manifest::Manifest;

static EXCLUDED_GLOBS: &[&str] = &[".*", "wally.lock", "Packages", "ServerPackages"];

/// Container for the contents of a package that have been downloaded.
#[derive(Clone)]
pub struct PackageContents {
    /// Contains a zip with the contents of the package.
    data: Vec<u8>,
}

impl PackageContents {
    pub fn pack_from_path(input: &Path) -> anyhow::Result<Self> {
        let manifest = Manifest::load(input)?;
        let package_name = manifest.package.name.name();

        let mut data = Vec::new();
        let mut archive = ZipWriter::new(Cursor::new(&mut data));

        for path in Self::filtered_contents(input)? {
            let relative_path = path.strip_prefix(input).unwrap();
            let archive_name = relative_path.to_str().ok_or_else(|| {
                format_err!(
                    "Path {} contained invalid Unicode characters",
                    relative_path.display()
                )
            })?;

            // Zips embed \ from windows paths causing incorrect extraction on unix operating
            // systems; we must sanitise here. See: https://github.com/UpliftGames/wally/issues/15
            // This may be fixed in the zip crate. See: https://github.com/zip-rs/zip/issues/253
            let archive_name = str::replace(archive_name, "\\", "/");

            if path.is_dir() {
                archive.add_directory(archive_name, FileOptions::default())?;
            } else {
                archive.start_file(archive_name, FileOptions::default())?;

                if path.ends_with("default.project.json") {
                    let project_file = File::open(path)?;
                    let mut project_json: serde_json::Value =
                        serde_json::from_reader(project_file)?;
                    let project_name = project_json
                        .get("name")
                        .and_then(|name| name.as_str())
                        .expect("Couldn't parse name in default.project.json");

                    if project_name != package_name {
                        log::info!(
                            "The project and package names are mismatched. The project name in \
                            `default.project.json` has been renamed to '{}' in the uploaded package \
                            to match the name provided by `wally.toml`",
                            package_name
                        );

                        *project_json.get_mut("name").unwrap() = json!(package_name);
                    }

                    serde_json::to_writer_pretty(&mut archive, &project_json)?;
                } else {
                    let mut file = BufReader::new(File::open(path)?);
                    io::copy(&mut file, &mut archive)?;
                }
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

    pub fn filtered_contents(input: &Path) -> anyhow::Result<Vec<PathBuf>> {
        let manifest = Manifest::load(input)?;
        let includes = manifest.package.include;
        let mut excludes = manifest.package.exclude;

        if includes.is_empty() && Path::new(".gitignore").exists() {
            let gitignore = File::open(Path::new(".gitignore"))?;

            BufReader::new(gitignore)
                .lines()
                .flatten()
                .for_each(|pattern| {
                    excludes.push(pattern);
                });
        }

        EXCLUDED_GLOBS
            .iter()
            .map(|pattern| pattern.to_string())
            .for_each(|pattern| excludes.push(pattern));

        let include = build_glob_set(&includes)?;
        let exclude = build_glob_set(&excludes)?;

        Ok(WalkDir::new(input)
            .min_depth(1)
            .into_iter()
            .filter_entry(|entry| {
                let relative = entry.path().strip_prefix(input).unwrap();

                if !includes.is_empty() && !include.matches(relative).is_empty() {
                    return true;
                };

                exclude.matches(relative).is_empty()
            })
            .flatten()
            .map(|entry| entry.path().to_path_buf())
            .collect())
    }

    pub fn data(&self) -> &[u8] {
        &self.data
    }

    /// Create a new PackageContents object from a buffer.
    pub fn from_buffer(data: Vec<u8>) -> PackageContents {
        PackageContents { data }
    }
}

fn build_glob_set(patterns: &[String]) -> anyhow::Result<GlobSet> {
    let mut builder = GlobSetBuilder::new();

    for pattern in patterns {
        builder.add(Glob::new(pattern)?);
    }

    Ok(builder.build()?)
}
