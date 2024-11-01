use std::fmt;
use std::str::FromStr;

use anyhow::{anyhow, ensure};
use serde::de::{Deserialize, Deserializer, Error, Visitor};
use serde::ser::{Serialize, Serializer};

/// Refers to a package, but not a specific version. Package names consist of a
/// scope and name.
///
/// Both the scope and name portions of a package name must consist only of
/// lowercase letters, digits, and dashes (`-`).
///
/// Examples of package names:
/// * `hello/world`
/// * `osyrisrblx/t`
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PackageName {
    // Fields are private here to enforce invariants around what characters are
    // valid in package names and scopes.
    scope: String,
    name: String,
}

impl PackageName {
    pub fn new<S, N>(scope: S, name: N) -> anyhow::Result<Self>
    where
        S: Into<String>,
        N: Into<String>,
    {
        let scope = scope.into();
        let name = name.into();

        validate_scope(&scope)?;
        validate_name(&name)?;

        Ok(PackageName { scope, name })
    }

    pub fn scope(&self) -> &str {
        &self.scope
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

fn validate_scope(scope: &str) -> anyhow::Result<()> {
    let only_valid_chars = scope
        .chars()
        .all(|char| char.is_ascii_lowercase() || char.is_ascii_digit() || char == '-');

    ensure!(
        only_valid_chars,
        "package scope '{}' is invalid (scopes can only contain lowercase characters, digits and \
         '-')",
        scope
    );
    ensure!(scope.len() > 0, "package scopes cannot be empty");
    ensure!(
        scope.len() <= 64,
        "package scopes cannot exceed 64 characters in length"
    );

    Ok(())
}

fn validate_name(name: &str) -> anyhow::Result<()> {
    let only_valid_chars = name
        .chars()
        .all(|char| char.is_ascii_lowercase() || char.is_ascii_digit() || char == '-');

    ensure!(
        only_valid_chars,
        "package name '{}' is invalid (names can only contain lowercase characters, digits and \
         '-')",
        name
    );
    ensure!(name.len() > 0, "package names cannot be empty");
    ensure!(
        name.len() <= 64,
        "package names cannot exceed 64 characters in length"
    );

    Ok(())
}

impl fmt::Display for PackageName {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "{}/{}", self.scope, self.name)
    }
}

impl FromStr for PackageName {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> anyhow::Result<Self> {
        const WRONG_NUMBER_ERR: &str = "a package name is of the form SCOPE/NAME";

        let mut pieces = value.splitn(2, '/');
        let scope = pieces.next().ok_or_else(|| anyhow!(WRONG_NUMBER_ERR))?;
        let name = pieces.next().ok_or_else(|| anyhow!(WRONG_NUMBER_ERR))?;

        PackageName::new(scope.to_owned(), name.to_owned())
    }
}

impl Serialize for PackageName {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let combined_name = format!("{}/{}", self.scope, self.name);
        serializer.serialize_str(&combined_name)
    }
}

impl<'de> Deserialize<'de> for PackageName {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_str(PackageNameVisitor)
    }
}

struct PackageNameVisitor;

impl<'de> Visitor<'de> for PackageNameVisitor {
    type Value = PackageName;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "a package name of the form SCOPE/NAME")
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
        let package = PackageName::new("flub-flab", "sisyphus-simulator-2").unwrap();
        assert_eq!(package.scope(), "flub-flab");
        assert_eq!(package.name(), "sisyphus-simulator-2");
    }

    #[test]
    fn new_invalid() {
        // Uppercase letters are not allowed.
        assert!(PackageName::new("Upper-Skewer-Case", "Foo").is_err());

        // Underscores are not allowed to prevent confusion with dashes.
        assert!(PackageName::new("snake_case", "foo").is_err());

        // Slashes are not allowed to avoid ambiguity.
        assert!(PackageName::new("hello/world", "from/me").is_err());

        // Scopes and names must have one or more characters.
        assert!(PackageName::new("", "").is_err());
    }

    #[test]
    fn parse() {
        let adopt_me: PackageName = "flub-flab/sisyphus-simulator".parse().unwrap();
        assert_eq!(adopt_me.scope(), "flub-flab");
        assert_eq!(adopt_me.name(), "sisyphus-simulator");

        let numbers: PackageName = "123/456".parse().unwrap();
        assert_eq!(numbers.scope(), "123");
        assert_eq!(numbers.name(), "456");
    }

    #[test]
    fn parse_invalid() {
        // Extra slashes should result in an error
        let extra: Result<PackageName, _> = "hello/world/foo".parse();
        assert!(extra.is_err());
    }

    #[test]
    fn display() {
        let package_name = PackageName::new("evaera", "promise").unwrap();
        assert_eq!(package_name.to_string(), "evaera/promise");
    }

    #[test]
    fn serialization() {
        let package_name = PackageName::new("lpghatguy", "asink").unwrap();
        let serialized = serde_json::to_string(&package_name).unwrap();
        assert_eq!(serialized, "\"lpghatguy/asink\"");

        let deserialized: PackageName = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized, package_name);
    }
}
