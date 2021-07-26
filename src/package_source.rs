mod in_memory;
mod registry;
mod test_registry;

pub use self::in_memory::InMemoryRegistry;
pub use self::registry::Registry;
pub use self::test_registry::TestRegistry;

use std::collections::HashMap;
use std::path::PathBuf;

use url::Url;

use crate::manifest::Manifest;
use crate::package_contents::PackageContents;
use crate::package_id::PackageId;
use crate::package_req::PackageReq;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PackageSourceId {
    DefaultRegistry,
    Git(Url),
    Path(PathBuf),
}

pub struct PackageSourceMap {
    sources: HashMap<PackageSourceId, Box<dyn PackageSource>>,
}

impl PackageSourceMap {
    pub fn new(default_registry: Box<dyn PackageSource>) -> Self {
        let mut sources = HashMap::new();
        sources.insert(PackageSourceId::DefaultRegistry, default_registry);

        Self { sources }
    }

    pub fn get(&self, id: &PackageSourceId) -> Option<&dyn PackageSource> {
        self.sources.get(id).map(|source| source.as_ref())
    }
}

pub trait PackageSource {
    /// Update this package source, if it has state that needs to be updated.
    fn update(&self) -> anyhow::Result<()>;

    /// Query this package source for all of the packages that match this
    /// `PackageReq`.
    fn query(&self, package_req: &PackageReq) -> anyhow::Result<Vec<Manifest>>;

    /// Downloads the contents of a package given its fully-qualified
    /// `PackageId`.
    fn download_package(&self, package_id: &PackageId) -> anyhow::Result<PackageContents>;
}
