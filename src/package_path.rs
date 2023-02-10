use std::path;

use std::fmt;
use std::str::FromStr;

use anyhow::bail;
use anyhow::Context;
use semver::VersionReq;
use serde::de::{Deserialize, Deserializer, Error, Visitor};
use serde::ser::{Serialize, Serializer};

use crate::manifest::Manifest;
use crate::package_req::PackageReq;

/// A package path is a path to a valid package.
/// Contains the path to the package.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PackagePath {
    // TODO: Consider only allowing relative paths?
    pub path: path::PathBuf,
}

impl PackagePath {
    pub fn new(path: path::PathBuf) -> Self {
        Self { path }
    }

    fn get_manifest_relative(&self, project_root: &path::Path) -> anyhow::Result<Manifest> {
        Manifest::load(&project_root.join(&self.path))
    }

    pub fn get_package_req(&self, project_root: &path::Path) -> anyhow::Result<PackageReq> {
        let manifest = self.get_manifest_relative(project_root)?;

        Ok(PackageReq::new(
            manifest.package.name.clone(),
            VersionReq::exact(&manifest.package.version),
        ))
    }
}

impl fmt::Display for PackagePath {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "fs+{}", self.path.to_string_lossy())
    }
}

impl FromStr for PackagePath {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> anyhow::Result<Self> {
        let (package_location_type, path) = value
            .split_once("+")
            .context("Expected a delimiter of '+'")?;

        if package_location_type != "fs" {
            bail!(format!(
                "Package location is not fs as expected, but instead is {}",
                package_location_type
            ))
        }

        Ok(Self::new(path.parse()?))
    }
}

impl Serialize for PackagePath {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for PackagePath {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_str(PackagePathVisitor)
    }
}

struct PackagePathVisitor;

impl<'de> Visitor<'de> for PackagePathVisitor {
    type Value = PackagePath;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "a package path which is a valid path")
    }

    fn visit_str<E: Error>(self, value: &str) -> Result<Self::Value, E> {
        value.parse().map_err(|err| E::custom(err))
    }
}

#[cfg(test)]
mod test {
    use std::path::PathBuf;

    use super::*;

    #[test]
    fn new() {
        let path = PathBuf::from("test/minimal");

        let package_path = PackagePath::new(path.clone());
        assert_eq!(package_path.path, path);
    }

    #[test]
    fn display() {
        let path = PathBuf::from("test/minimal");
        let package_path = PackagePath::new(path);

        assert_eq!(package_path.to_string(), "fs+test/minimal");
    }

    #[test]
    fn parse() {
        let package_path: PackagePath = "fs+hello/world".parse().unwrap();

        assert_eq!(package_path.path.to_str().unwrap(), "hello/world");
    }

    #[test]
    fn parse_invalid() {
        // Requires a '+' to make sure it isn't conflated with a PackageReq.
        "hello/world".parse::<PackagePath>().unwrap_err();
        // Requires a 'fs+' specifically.
        "git+hello/world".parse::<PackagePath>().unwrap_err();
    }

    #[test]
    fn serialization() {
        let package_path: PackagePath = "fs+hello/world/among".parse().unwrap();

        let serialized = serde_json::to_string(&package_path).unwrap();
        assert_eq!(serialized, r#""fs+hello/world/among""#);

        let deserialized: PackagePath = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized, package_path)
    }
}
