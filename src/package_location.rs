use crate::package_id::PackageId;
use crate::package_name::PackageName;
use crate::package_path::PackagePath;
use crate::package_req::PackageReq;
use semver::Version;
use semver::VersionReq;
use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Clone, Serialize, Deserialize, PartialOrd, Ord, PartialEq, Eq)]
#[serde(untagged)]
pub enum PackageLocation {
    Registry(PackageReq),
    Path(PackagePath),
}

// Super-hacky: make them into a trait heaven sakes!
impl PackageLocation {
    pub fn name(&self) -> &PackageName {
        match self {
            PackageLocation::Registry(pkg) => pkg.name(),
            PackageLocation::Path(pkg) => pkg.req.name(),
        }
    }

    pub fn version_req(&self) -> &VersionReq {
        match self {
            PackageLocation::Registry(pkg) => pkg.version_req(),
            PackageLocation::Path(pkg) => pkg.req.version_req(),
        }
    }

    pub fn matches_id(&self, package_id: &PackageId) -> bool {
        self.matches(package_id.name(), package_id.version())
    }

    pub fn matches(&self, name: &PackageName, version: &Version) -> bool {
        self.name() == name && self.version_req().matches(version)
    }
}