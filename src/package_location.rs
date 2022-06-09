use crate::package_id::PackageId;
use crate::package_name::PackageName;
use crate::package_origin::PackageOrigin;
use crate::package_path::PackagePath;
use crate::package_req::PackageReq;
use crate::package_source::PackageSourceId;
use semver::Version;
use semver::VersionReq;
use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Clone, Serialize, Deserialize, PartialOrd, Ord, PartialEq, Eq)]
#[serde(untagged)]
pub enum PackageLocation {
    Registry(PackageReq),
    // Path should always be the last enumeration as we want to try parsing a
    // PackagePath last! Doing it first will always succeed due to a path being
    // infallible.
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

    pub fn package_req(&self) -> PackageReq {
        PackageReq::new(self.name().clone(), self.version_req().clone())
    }

    pub fn matches_id(&self, package_id: &PackageId) -> bool {
        self.matches(package_id.name(), package_id.version())
    }

    pub fn package_origin(&self) -> PackageOrigin {
        match self {
            // TODO: PackageLocation does not encode the source registry. It only encodes the packageId itself.
            // Maybe change it so that it does contain the source registry.
            // Otherwise, package_origin could take an argument for the expected registry.
            // *sigh*
            PackageLocation::Registry(_) => PackageOrigin::Registry(PackageSourceId::DefaultRegistry),
            PackageLocation::Path(PackagePath { path, .. }) => {
                PackageOrigin::Path(path.clone())
            }
        }
    }

    pub fn matches(&self, name: &PackageName, version: &Version) -> bool {
        self.name() == name && self.version_req().matches(version)
    }
}
