use std::fmt;
use std::str::FromStr;

use anyhow::{anyhow, Context};
use semver::Version;
use serde::de::{Deserialize, Deserializer, Error, Visitor};
use serde::ser::{Serialize, Serializer};

use crate::package_name::PackageName;

/// Refers to a specific version of a package. Package IDs consist of a scope,
/// name, and SemVer version.
///
/// All rules for package names apply to the first portion of a package ID.
///
/// Examples of package IDs:
/// * `hello/world@1.2.3`
/// * `miss-frizz/magic-school-bus@0.2.3-pre1+build102312`
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PackageId {
    name: PackageName,
    version: Version,
}

impl PackageId {
    pub fn new(name: PackageName, version: Version) -> Self {
        Self { name, version }
    }

    pub fn name(&self) -> &PackageName {
        &self.name
    }

    pub fn version(&self) -> &Version {
        &self.version
    }

    pub fn into_parts(self) -> (PackageName, Version) {
        (self.name, self.version)
    }
}

impl fmt::Display for PackageId {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "{}@{}", self.name, self.version)
    }
}

impl FromStr for PackageId {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> anyhow::Result<Self> {
        const BAD_FORMAT_MSG: &str = "a package ID is of the form SCOPE/NAME@VERSION";

        let mut first_half = value.splitn(2, '/');
        let scope = first_half.next().ok_or_else(|| anyhow!(BAD_FORMAT_MSG))?;
        let name_and_version = first_half.next().ok_or_else(|| anyhow!(BAD_FORMAT_MSG))?;

        let mut second_half = name_and_version.splitn(2, '@');
        let name = second_half.next().ok_or_else(|| anyhow!(BAD_FORMAT_MSG))?;
        let version = second_half
            .next()
            .ok_or_else(|| anyhow!(BAD_FORMAT_MSG))?
            .parse()
            .context("could not parse version")?;

        let package_name = PackageName::new(scope, name).context(BAD_FORMAT_MSG)?;
        Ok(PackageId::new(package_name, version))
    }
}

impl Serialize for PackageId {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let combined_name = format!(
            "{}/{}@{}",
            self.name().scope(),
            self.name().name(),
            self.version()
        );
        serializer.serialize_str(&combined_name)
    }
}

impl<'de> Deserialize<'de> for PackageId {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_str(PackageIdVisitor)
    }
}

struct PackageIdVisitor;

impl<'de> Visitor<'de> for PackageIdVisitor {
    type Value = PackageId;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "a package ID of the form SCOPE/NAME@VERSION")
    }

    fn visit_str<E: Error>(self, value: &str) -> Result<Self::Value, E> {
        value.parse().map_err(|err| E::custom(err))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn new() {
        let id = PackageId::new(
            PackageName::new("foo", "bar").unwrap(),
            Version::new(1, 2, 3),
        );
        assert_eq!(id.name().scope(), "foo");
        assert_eq!(id.name().name(), "bar");
        assert_eq!(id.version(), &Version::new(1, 2, 3));
    }

    #[test]
    fn display() {
        let id = PackageId::new(
            PackageName::new("hello", "world").unwrap(),
            Version::new(0, 2, 3),
        );
        assert_eq!(id.to_string(), "hello/world@0.2.3");
    }

    #[test]
    fn parse() {
        let hello: PackageId = "hello/world@1.2.3".parse().unwrap();
        assert_eq!(hello.name().scope(), "hello");
        assert_eq!(hello.name().name(), "world");
        assert_eq!(hello.version(), &Version::new(1, 2, 3));
    }

    #[test]
    fn parse_invalid() {
        // Package IDs require a version.
        let no_version: Result<PackageId, _> = "hello/world".parse();
        no_version.unwrap_err();
        let no_version_at: Result<PackageId, _> = "hello/world@".parse();
        no_version_at.unwrap_err();

        // Incomplete versions are not allowed.
        let not_enough_version: Result<PackageId, _> = "foo/bar@2".parse();
        not_enough_version.unwrap_err();
    }

    #[test]
    fn serialization() {
        let name = PackageName::new("lpghatguy", "asink").unwrap();
        let package_id = PackageId::new(name, Version::new(2, 3, 1));

        let serialized = serde_json::to_string(&package_id).unwrap();
        assert_eq!(serialized, "\"lpghatguy/asink@2.3.1\"");

        let deserialized: PackageId = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized, package_id);
    }
}
