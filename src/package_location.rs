use std::path;

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
    Path(PackagePath),
}

// Helpers for resolution.
impl PackageLocation {
    pub fn name(&self, project_root: &path::Path) -> anyhow::Result<PackageName> {
        Ok(match self {
            PackageLocation::Registry(pkg) => pkg.name().clone(),
            PackageLocation::Path(pkg) => pkg.get_package_req(project_root)?.name().clone(),
        })
    }

    pub fn version_req(&self, project_root: &path::Path) -> anyhow::Result<VersionReq> {
        Ok(match self {
            PackageLocation::Registry(pkg) => pkg.version_req().clone(),
            PackageLocation::Path(pkg) => pkg.get_package_req(project_root)?.version_req().clone(),
        })
    }

    pub fn package_req(&self, project_root: &path::Path) -> anyhow::Result<PackageReq> {
        Ok(PackageReq::new(
            self.name(project_root)?.clone(),
            self.version_req(project_root)?.clone(),
        ))
    }

    pub fn matches_id(
        &self,
        package_id: &PackageId,
        project_root: &path::Path,
    ) -> anyhow::Result<bool> {
        self.matches(package_id.name(), package_id.version(), project_root)
    }

    pub fn package_origin(&self) -> PackageOrigin {
        match self {
            // TODO: PackageLocation does not encode the source registry. It only encodes the packageId itself.
            // Maybe change it so that it does contain the source registry?
            // Otherwise, package_origin could take an argument for the expected registry.
            PackageLocation::Registry(_) => {
                PackageOrigin::Registry(PackageSourceId::DefaultRegistry)
            }
            PackageLocation::Path(PackagePath { path, .. }) => PackageOrigin::Path(path.clone()),
        }
    }

    pub fn matches(
        &self,
        name: &PackageName,
        version: &Version,
        project_root: &path::Path,
    ) -> anyhow::Result<bool> {
        Ok(self.name(project_root)? == *name && self.version_req(project_root)?.matches(version))
    }
}
