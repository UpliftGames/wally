use std::path::PathBuf;

use anyhow::{bail, Context};

use crate::manifest::{Manifest, MANIFEST_FILE_NAME};
use crate::package_contents::PackageContents;
use crate::package_id::PackageId;
use crate::package_req::PackageReq;

use super::{PackageSourceId, PackageSourceProvider};

/// A package source backed by a local filesystem directory containing a
/// `wally.toml`. Used for workspace path dependencies.
#[derive(Debug, Clone)]
pub struct PathSource {
    root: PathBuf,
}

impl PathSource {
    pub fn new(root: PathBuf) -> Self {
        PathSource { root }
    }

    pub fn root(&self) -> &PathBuf {
        &self.root
    }

    fn load_manifest(&self) -> anyhow::Result<Manifest> {
        if !self.root.is_dir() {
            bail!(
                "path source directory does not exist: {}",
                self.root.display()
            );
        }
        if !self.root.join(MANIFEST_FILE_NAME).exists() {
            bail!(
                "path source directory has no {}: {}",
                MANIFEST_FILE_NAME,
                self.root.display()
            );
        }
        Manifest::load(&self.root)
            .with_context(|| format!("failed to load manifest from {}", self.root.display()))
    }
}

impl PackageSourceProvider for PathSource {
    fn update(&self) -> anyhow::Result<()> {
        Ok(())
    }

    fn query(&self, package_req: &PackageReq) -> anyhow::Result<Vec<Manifest>> {
        let manifest = self.load_manifest()?;
        if package_req.matches(&manifest.package.name, &manifest.package.version) {
            Ok(vec![manifest])
        } else {
            Ok(Vec::new())
        }
    }

    fn download_package(&self, _package_id: &PackageId) -> anyhow::Result<PackageContents> {
        PackageContents::pack_from_path(&self.root)
            .with_context(|| format!("failed to pack path source at {}", self.root.display()))
    }

    fn fallback_sources(&self) -> anyhow::Result<Vec<PackageSourceId>> {
        Ok(Vec::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn write_package(dir: &std::path::Path, manifest: &str) {
        fs::create_dir_all(dir).unwrap();
        fs::write(dir.join("wally.toml"), manifest).unwrap();
        // Create a minimal default.project.json and src/init.lua so
        // pack_from_path succeeds.
        fs::write(
            dir.join("default.project.json"),
            r#"{"name":"test","tree":{"$path":"src"}}"#,
        )
        .unwrap();
        let src = dir.join("src");
        fs::create_dir_all(&src).unwrap();
        fs::write(src.join("init.lua"), "return {}").unwrap();
    }

    #[test]
    fn query_matching_package() {
        let tmp = TempDir::new().unwrap();
        write_package(
            tmp.path(),
            r#"
            [package]
            name = "team/foo"
            version = "1.2.3"
            registry = "https://example.com/index"
            realm = "shared"
        "#,
        );

        let source = PathSource::new(tmp.path().to_path_buf());
        let req: PackageReq = "team/foo@1.2.3".parse().unwrap();
        let results = source.query(&req).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].package.name.to_string(), "team/foo");
    }

    #[test]
    fn query_non_matching_version() {
        let tmp = TempDir::new().unwrap();
        write_package(
            tmp.path(),
            r#"
            [package]
            name = "team/foo"
            version = "1.2.3"
            registry = "https://example.com/index"
            realm = "shared"
        "#,
        );

        let source = PathSource::new(tmp.path().to_path_buf());
        let req: PackageReq = "team/foo@2.0.0".parse().unwrap();
        let results = source.query(&req).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn query_non_matching_name() {
        let tmp = TempDir::new().unwrap();
        write_package(
            tmp.path(),
            r#"
            [package]
            name = "team/foo"
            version = "1.0.0"
            registry = "https://example.com/index"
            realm = "shared"
        "#,
        );

        let source = PathSource::new(tmp.path().to_path_buf());
        let req: PackageReq = "team/bar@1.0.0".parse().unwrap();
        let results = source.query(&req).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn download_package_produces_valid_contents() {
        let tmp = TempDir::new().unwrap();
        write_package(
            tmp.path(),
            r#"
            [package]
            name = "team/packable"
            version = "1.0.0"
            registry = "https://example.com/index"
            realm = "shared"
        "#,
        );

        let source = PathSource::new(tmp.path().to_path_buf());
        let id = PackageId::new(
            "team/packable".parse().unwrap(),
            "1.0.0".parse().unwrap(),
        );
        let contents = source.download_package(&id).unwrap();
        assert!(!contents.data().is_empty());

        // Verify it can be unpacked
        let out = TempDir::new().unwrap();
        contents.unpack_into_path(out.path()).unwrap();
        assert!(out.path().join("src").join("init.lua").exists());
    }

    #[test]
    fn error_path_does_not_exist() {
        let source = PathSource::new(PathBuf::from("/nonexistent/path/to/pkg"));
        let req: PackageReq = "team/foo@1.0.0".parse().unwrap();
        let result = source.query(&req);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("does not exist"),
            "error should mention non-existence: {}",
            err
        );
    }

    #[test]
    fn error_path_has_no_manifest() {
        let tmp = TempDir::new().unwrap();
        // Directory exists but has no wally.toml
        let source = PathSource::new(tmp.path().to_path_buf());
        let req: PackageReq = "team/foo@1.0.0".parse().unwrap();
        let result = source.query(&req);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("wally.toml"),
            "error should mention missing manifest: {}",
            err
        );
    }
}
