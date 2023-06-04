//! Defines a package registry that can be defined and modified entirely in
//! memory. It's useful for creating exact conditions for test cases for
//! resolution, installation, upgrading, etc.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::RwLock;

use anyhow::format_err;

use crate::{
    manifest::Manifest, package_id::PackageId, package_req::PackageReq,
    package_source::PackageSource, test_package::PackageBuilder,
};

use super::{PackageContents, PackageSourceId, PackageSourceProvider};

/// An in-memory registry that can have packages published to it.
///
/// `InMemoryRegistry` itself is not a `PackageSource`, but one can be created
/// by calling `source()`.
pub struct InMemoryRegistry {
    storage: Storage,
}

impl InMemoryRegistry {
    pub fn new() -> Self {
        Self {
            storage: Default::default(),
        }
    }

    /// Publish a new package to the registry.
    pub fn publish(&self, builder: PackageBuilder) {
        let mut storage = self.storage.contents.write().unwrap();
        let (manifest, contents) = builder.package();

        let scope = storage
            .entry(manifest.package.name.scope().to_owned())
            .or_default();

        let entries = scope
            .entry(manifest.package.name.name().to_owned())
            .or_default();

        entries.push(PackageEntry { manifest, contents });
    }

    /// Returns a handle to an object that can be used as a `PackageSource`.
    pub fn source(&self) -> PackageSource {
        PackageSource::InMemory(InMemoryRegistrySource {
            storage: self.storage.clone(),
        })
    }
}

/// Returned by `InMemoryRegistry::source` and can be passed to package
/// resolution code in order to tell it to use this package registry.
#[derive(Clone)]
pub struct InMemoryRegistrySource {
    storage: Storage,
}

impl PackageSourceProvider for InMemoryRegistrySource {
    fn update(&self) -> anyhow::Result<()> {
        Ok(())
    }

    fn query(&self, package_req: &PackageReq) -> anyhow::Result<Vec<Manifest>> {
        let storage = self.storage.contents.read().unwrap();
        let scope = match storage.get(package_req.name().scope()) {
            Some(scope) => scope,
            None => return Ok(Vec::new()),
        };

        let entries = match scope.get(package_req.name().name()) {
            Some(entries) => entries,
            None => return Ok(Vec::new()),
        };

        let result = entries
            .iter()
            .filter(|entry| {
                package_req
                    .version_req()
                    .matches(&entry.manifest.package.version)
            })
            .map(|entry| &entry.manifest)
            .cloned()
            .collect();

        Ok(result)
    }

    fn download_package(&self, package_id: &PackageId) -> anyhow::Result<PackageContents> {
        let storage = self.storage.contents.read().unwrap();
        let scope = storage
            .get(package_id.name().scope())
            .ok_or_else(|| format_err!("Package {} does not exist", package_id))?;

        let manifests = scope
            .get(package_id.name().name())
            .ok_or_else(|| format_err!("Package {} does not exist", package_id))?;

        let entry = manifests
            .iter()
            .find(|entry| &entry.manifest.package.version == package_id.version())
            .ok_or_else(|| format_err!("Package {} does not exist", package_id))?;

        Ok(entry.contents.clone())
    }

    fn fallback_sources(&self) -> anyhow::Result<Vec<PackageSourceId>> {
        todo!("Implement in-memory fallback sources");
    }
}

struct PackageEntry {
    manifest: Manifest,
    contents: PackageContents,
}

#[derive(Clone, Default)]
struct Storage {
    contents: Arc<RwLock<HashMap<String, HashMap<String, Vec<PackageEntry>>>>>,
}
