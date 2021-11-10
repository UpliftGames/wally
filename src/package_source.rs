mod in_memory;
mod registry;
mod test_registry;

pub use self::in_memory::InMemoryRegistry;
pub use self::registry::Registry;
pub use self::test_registry::TestRegistry;

use std::collections::HashMap;
use std::path::PathBuf;

use serde::Serialize;

use crate::manifest::Manifest;
use crate::package_contents::PackageContents;
use crate::package_id::PackageId;
use crate::package_req::PackageReq;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub enum PackageSourceId {
    DefaultRegistry,
    Git(String),
    Path(PathBuf),
}

pub struct PackageSourceMap {
    sources: HashMap<PackageSourceId, Box<dyn PackageSource>>,
    source_order: Vec<PackageSourceId>,
}

impl PackageSourceMap {
    pub fn new(default_registry: Box<dyn PackageSource>) -> Self {
        let mut sources = HashMap::new();
        sources.insert(PackageSourceId::DefaultRegistry, default_registry);

        Self {
            sources,
            source_order: vec![PackageSourceId::DefaultRegistry],
        }
    }

    pub fn get(&self, id: &PackageSourceId) -> Option<&dyn PackageSource> {
        self.sources.get(id).map(|source| source.as_ref())
    }

    pub fn source_order(&self) -> &Vec<PackageSourceId> {
        &self.source_order
    }

    /// Searches the current list of sources for fallbacks and adds any not yet in the list, producing
    /// a complete tree of reachable sources for packages.
    /// Sources are searched breadth-first to ensure correct fallback priority.
    pub fn add_fallbacks(&mut self) -> anyhow::Result<()> {
        let mut source_index = 0;

        while source_index < self.source_order.len() {
            let registry = self.sources.get(&self.source_order[source_index]).unwrap();

            for fallback in registry.fallback_sources()? {
                // Prevent circular references by only adding new sources
                if !self.source_order.contains(&fallback) {
                    let source: Box<dyn PackageSource> = match &fallback {
                        PackageSourceId::Git(url) => Box::new(Registry::from_registry_spec(url)?),
                        PackageSourceId::Path(path) => Box::new(TestRegistry::new(path.clone())),
                        PackageSourceId::DefaultRegistry => {
                            panic!("Default registry should never be added as a fallback source!")
                        }
                    };

                    self.sources.insert(fallback.clone(), source);
                    self.source_order.push(fallback);
                }
            }

            source_index += 1;
        }

        Ok(())
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

    /// Provide a list of fallback sources to search if this source can't provide a package
    fn fallback_sources(&self) -> anyhow::Result<Vec<PackageSourceId>>;
}
