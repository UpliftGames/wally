use crate::{manifest::Realm, package_name::PackageName, package_req::PackageReq};
use std::str::FromStr;

#[derive(Debug, PartialEq, Eq, Hash)]
pub enum PackageSpec {
    Named(PackageName),
    Required(PackageReq),
}

impl PackageSpec {
    pub fn name(&self) -> &str {
        match self {
            PackageSpec::Named(named) => named.name(),
            PackageSpec::Required(required) => required.name().name(),
        }
    }
}

impl FromStr for PackageSpec {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> anyhow::Result<Self> {
        if let Ok(package_req) = value.parse() {
            Ok(PackageSpec::Required(package_req))
        } else if let Ok(package_name) = value.parse() {
            Ok(PackageSpec::Named(package_name))
        } else {
            anyhow::bail!(
                "Was unable to parse {} into a package requirement or a package name!",
                value
            )
        }
    }
}

pub fn as_table_name(realm: &Realm) -> &'static str {
    match realm {
        Realm::Server => "server-dependencies",
        Realm::Shared => "dependencies",
        Realm::Dev => "dev-dependencies",
    }
}
