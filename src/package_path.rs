use std::path;

use std::fmt;
use std::str::FromStr;

use semver::VersionReq;

use serde::de::{Deserialize, Deserializer, Error, Visitor};
use serde::ser::{Serialize, Serializer};

use crate::manifest::Manifest;
use crate::package_req::PackageReq;

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PackagePath {
    pub path: path::PathBuf,
    pub req: PackageReq,
}

// TODO: Make PackagePath use `fs+`
impl PackagePath {
    fn new(path: path::PathBuf) -> anyhow::Result<Self> {
        let manifest = Manifest::load(&path)?;

        let package_req = PackageReq::new(
            manifest.package_id().name().to_owned(),
            VersionReq::exact(manifest.package_id().version()),
        );

        Ok(Self { path, req: package_req })
    }
}

impl fmt::Display for PackagePath {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "{}", self.path.to_string_lossy())
    }
}

impl FromStr for PackagePath {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> anyhow::Result<Self> {
        Self::new(value.parse()?)
    }
}

impl Serialize for PackagePath {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.path.to_string_lossy())
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
// TODO: Test for version!
mod test {
    use super::*;
    use std::path::PathBuf;

    fn from_test_path_str(path: &str) -> String {
        env!("CARGO_MANIFEST_DIR").to_owned() + "/test-projects/" + path + "/"
    }

    fn from_test_path(path: &str) -> PathBuf {
        PathBuf::from(from_test_path_str(path))
    }

    #[test]
    fn new() {
        let package_path = PackagePath::new(from_test_path("minimal")).unwrap();
        assert_eq!(package_path.path, from_test_path("minimal"));
    }

    #[test]
    fn display() {
        let package_path = PackagePath::new(from_test_path("one-dependency")).unwrap();
        assert_eq!(
            package_path.to_string(),
            from_test_path_str("one-dependency")
        );
    }

    #[test]
    fn parse() {
        let package_path: PackagePath = from_test_path_str("minimal").parse().unwrap();

        assert_eq!(package_path.to_string(), from_test_path_str("minimal"));
    }

    #[test]
    fn parse_invalid() {
        // Conversion is infalliable and thus, there is never an invaild parse.
        // https://doc.rust-lang.org/src/std/path.rs.html#1684
        // But, we can still test for invaild paths.

        let _ = from_test_path_str("does-not-exist")
            .parse::<PackagePath>()
            .unwrap_err();
    }

    #[test]
    #[ignore = "Not sure how to handle the mixing between Windows path and unix paths."]
    fn serialization() {
        let package_path: PackagePath = from_test_path_str("minimal").parse().unwrap();

        let serialized = serde_json::to_string(&package_path).unwrap();
        assert_eq!(serialized, "\"".to_owned() + &from_test_path_str("minimal") + "\"");

        let deserialized: PackagePath = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized, package_path)
    }
}
