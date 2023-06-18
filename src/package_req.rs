use std::fmt;
use std::str::FromStr;

use anyhow::{anyhow, bail, Context};
use semver::{Op, Version, VersionReq};
use serde::de::{Deserialize, Deserializer, Error, Visitor};
use serde::ser::{Serialize, Serializer};

use crate::package_id::PackageId;
use crate::package_name::PackageName;

/// Describes a requirement on a package, consisting of a scope, name, and valid
/// version range.
///
/// Examples of package requirements:
/// * `roblox/roact@1.4.2`
/// * `lpghatguy/asink@0.2.0-alpha.3`
/// * `foo/bar@1`
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PackageReq {
    name: PackageName,
    version_req: VersionReq,
}

impl PackageReq {
    pub fn new(name: PackageName, version_req: VersionReq) -> Self {
        PackageReq { name, version_req }
    }

    pub fn name(&self) -> &PackageName {
        &self.name
    }

    pub fn version_req(&self) -> &VersionReq {
        &self.version_req
    }

    pub fn matches_id(&self, package_id: &PackageId) -> bool {
        self.matches(package_id.name(), package_id.version())
    }

    pub fn matches(&self, name: &PackageName, version: &Version) -> bool {
        self.name() == name && self.version_req.matches(version)
    }
}

impl fmt::Display for PackageReq {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        let version_req = self.version_req().to_string();

        write!(
            formatter,
            "{}@{}",
            self.name,
            // Convention: The VersionReq ^1.1.1 should simplify to 1.1.1.
            match &self.version_req.comparators[..] {
                [comparator] if comparator.op == Op::Caret => version_req.trim_start_matches('^'),
                _ => &version_req,
            }
        )
    }
}

impl FromStr for PackageReq {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> anyhow::Result<Self> {
        const BAD_FORMAT_MSG: &str = "a package requirement is of the form SCOPE/NAME@VERSION_REQ";

        let mut first_half = value.splitn(2, '/');
        let scope = first_half.next().ok_or_else(|| anyhow!(BAD_FORMAT_MSG))?;
        let name_and_version = first_half.next().ok_or_else(|| anyhow!(BAD_FORMAT_MSG))?;

        let mut second_half = name_and_version.splitn(2, '@');
        let name = second_half.next().ok_or_else(|| anyhow!(BAD_FORMAT_MSG))?;

        let version_req_source = second_half.next().ok_or_else(|| anyhow!(BAD_FORMAT_MSG))?;

        // The VersionReq type will successfully parse from an empty or
        // all-spaces string, yielding a wildcard. This is not behavior we want,
        // so let's check for that here.
        //
        // https://github.com/steveklabnik/semver-parser/issues/51
        if version_req_source.len() == 0 || version_req_source.chars().all(char::is_whitespace) {
            bail!(BAD_FORMAT_MSG);
        }

        let version_req = version_req_source
            .parse()
            .context("could not parse version requirement")?;

        let package_name = PackageName::new(scope, name).context(BAD_FORMAT_MSG)?;
        Ok(PackageReq::new(package_name, version_req))
    }
}

impl Serialize for PackageReq {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let combined_name = self.to_string();
        serializer.serialize_str(&combined_name)
    }
}

impl<'de> Deserialize<'de> for PackageReq {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_str(PackageReqVisitor)
    }
}

struct PackageReqVisitor;

impl<'de> Visitor<'de> for PackageReqVisitor {
    type Value = PackageReq;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(
            formatter,
            "a package requirement of the form SCOPE/NAME@VERSION_REQ"
        )
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
        let req = PackageReq::new(
            PackageName::new("foo", "bar").unwrap(),
            VersionReq::parse("1.2.3").unwrap(),
        );
        assert_eq!(req.name().scope(), "foo");
        assert_eq!(req.name().name(), "bar");
        assert_eq!(req.version_req(), &VersionReq::parse("1.2.3").unwrap());
    }

    #[test]
    fn when_one_carot_predicate_omit_carot() {
        let req = PackageReq::new(
            PackageName::new("hello", "world").unwrap(),
            VersionReq::parse("0.2.3").unwrap(),
        );

        // We make sure that if there's only one predicate that's the '^', the carot is omitted.
        assert_eq!(req.to_string(), "hello/world@0.2.3");
    }

    #[test]
    fn parse() {
        // If given a semver version, we default to the ^ operator, which means
        // "compatible with". This is a good default that Cargo also chooses.
        let default_compat: PackageReq = "hello/world@1.2.3".parse().unwrap();
        assert_eq!(default_compat.name().scope(), "hello");
        assert_eq!(default_compat.name().name(), "world");
        assert_eq!(
            default_compat.version_req(),
            &VersionReq::parse("^1.2.3").unwrap()
        );

        // Arbitrarily complex semver predicates can be chained together. This
        // range might mean "0.2.7 is really broken and I don't want it".
        let with_ops: PackageReq = "hello/world@>=0.2.0, <0.2.7".parse().unwrap();
        assert_eq!(with_ops.name().scope(), "hello");
        assert_eq!(with_ops.name().name(), "world");
        assert_eq!(
            with_ops.version_req(),
            &VersionReq::parse(">=0.2.0, <0.2.7").unwrap()
        );
    }

    #[test]
    fn parse_invalid() {
        // Package requirements require a version requirement.
        let no_version: Result<PackageReq, _> = "hello/world".parse();
        no_version.unwrap_err();
        let no_version_at: Result<PackageReq, _> = "hello/world@".parse();
        no_version_at.unwrap_err();
    }

    #[test]
    fn serialization() {
        let name = PackageName::new("lpghatguy", "asink").unwrap();
        let package_req = PackageReq::new(name, VersionReq::parse("2.3.1").unwrap());

        let serialized = serde_json::to_string(&package_req).unwrap();
        assert_eq!(serialized, "\"lpghatguy/asink@2.3.1\"");

        let deserialized: PackageReq = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized, package_req);
    }
}
