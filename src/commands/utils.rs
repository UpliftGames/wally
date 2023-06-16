use crate::{package_name::PackageName, package_req::PackageReq};
use std::str::FromStr;

#[derive(Debug, PartialEq, Eq, Hash)]
pub enum PackageSpec {
    Named(PackageName),
    Required(PackageReq),
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
