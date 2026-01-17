use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet, VecDeque};

use anyhow::bail;
use anyhow::format_err;
use semver::Version;
use serde::Serialize;

use crate::manifest::{Manifest, Realm};
use crate::package_id::PackageId;
use crate::package_req::PackageReq;
use crate::package_source::{PackageSourceId, PackageSourceMap, PackageSourceProvider};

/// A completely resolved graph of packages returned by `resolve`.
///
/// State here is stored in multiple maps, all keyed by PackageId, to facilitate
/// concurrent mutable access to unrelated information about different packages.
#[derive(Debug, Default, Serialize, Clone)]
pub struct Resolve {
    /// Set of all packages that have been chosen to be part of the package
    /// graph.
    pub activated: BTreeSet<PackageId>,

    /// Metadata stored about each package that does not need to be accessed
    /// concurrently to other information.
    pub metadata: BTreeMap<PackageId, ResolvePackageMetadata>,

    /// Graph of all dependencies originating from the "shared" dependency realm.
    pub shared_dependencies: BTreeMap<PackageId, BTreeMap<String, PackageId>>,

    /// Graph of all dependencies originating from the "server" dependency realm.
    pub server_dependencies: BTreeMap<PackageId, BTreeMap<String, PackageId>>,

    /// Graph of all dependencies originating from the "dev" dependency realm.
    pub dev_dependencies: BTreeMap<PackageId, BTreeMap<String, PackageId>>,
}

impl Resolve {
    fn activate(&mut self, source: PackageId, dep_name: String, dep_realm: Realm, dep: PackageId) {
        self.activated.insert(dep.clone());

        let dependencies = match dep_realm {
            Realm::Shared => self.shared_dependencies.entry(source).or_default(),
            Realm::Server => self.server_dependencies.entry(source).or_default(),
            Realm::Dev => self.dev_dependencies.entry(source).or_default(),
        };
        dependencies.insert(dep_name, dep);
    }
}

/// A single node in the package resolution graph.
/// Origin realm is the "most restrictive" realm the package can still be dependended
/// upon. It is where the package gets placed during install.
/// See [ origin_realm clarification ]. In the resolve function for more info.
#[derive(Debug, Serialize, Clone)]
pub struct ResolvePackageMetadata {
    pub realm: Realm,
    pub origin_realm: Realm,
    pub source_registry: PackageSourceId,
}

pub fn resolve(
    root_manifest: &Manifest,
    try_to_use: &BTreeSet<PackageId>,
    package_sources: &PackageSourceMap,
) -> anyhow::Result<Resolve> {
    let mut resolve = Resolve::default();

    // Insert root project into graph and activated dependencies, as it'll
    // always be present.
    resolve.activated.insert(root_manifest.package_id());
    resolve.metadata.insert(
        root_manifest.package_id(),
        ResolvePackageMetadata {
            realm: root_manifest.package.realm,
            origin_realm: root_manifest.package.realm,
            source_registry: PackageSourceId::DefaultRegistry,
        },
    );

    // Queue of all dependency requests that need to be resolved.
    let mut packages_to_visit = VecDeque::new();

    for (alias, req) in &root_manifest.dependencies {
        packages_to_visit.push_back(DependencyRequest {
            request_source: root_manifest.package_id(),
            request_realm: Realm::Shared,
            origin_realm: Realm::Shared,
            package_alias: alias.clone(),
            package_req: req.clone(),
        });
    }

    for (alias, req) in &root_manifest.server_dependencies {
        packages_to_visit.push_back(DependencyRequest {
            request_source: root_manifest.package_id(),
            request_realm: Realm::Server,
            origin_realm: Realm::Server,
            package_alias: alias.clone(),
            package_req: req.clone(),
        });
    }

    for (alias, req) in &root_manifest.dev_dependencies {
        packages_to_visit.push_back(DependencyRequest {
            request_source: root_manifest.package_id(),
            request_realm: Realm::Dev,
            origin_realm: Realm::Dev,
            package_alias: alias.clone(),
            package_req: req.clone(),
        });
    }

    // Workhorse loop: resolve all dependencies, depth-first.
    'outer: while let Some(dependency_request) = packages_to_visit.pop_front() {
        // Locate all already-activated packages that might match this
        // dependency request.
        let mut matching_activated: Vec<_> = resolve
            .activated
            .iter()
            .filter(|package_id| package_id.name() == dependency_request.package_req.name())
            .cloned()
            .collect();

        // Sort our list of candidates by descending version so that we can pick
        // newest candidates first.
        matching_activated.sort_by(|a, b| b.version().cmp(a.version()));

        // Check for the highest version already-activated package that matches
        // our constraints.
        for package_id in &matching_activated {
            if dependency_request.package_req.matches_id(package_id) {
                let metadata = resolve
                    .metadata
                    .get_mut(package_id)
                    .expect("activated package was missing metadata");

                // [ origin_realm clarification ]
                // We want to set the origin to the most restrictive origin possible.
                // For example we want to keep packages in the dev realm unless a dependency
                // with a shared/server origin requires it. This way server/shared dependencies
                // which only originate from dev dependencies get put into the dev folder even
                // if they usually belong to another realm. Likewise we want to keep shared
                // dependencies in the server realm unless they are explicitly required as a
                // shared dependency.
                let realm_match = match (metadata.origin_realm, dependency_request.origin_realm) {
                    (_, Realm::Shared) => Realm::Shared,
                    (Realm::Shared, _) => Realm::Shared,
                    (_, Realm::Server) => Realm::Server,
                    (Realm::Server, _) => Realm::Server,
                    (Realm::Dev, Realm::Dev) => Realm::Dev,
                };

                metadata.origin_realm = realm_match;

                resolve.activate(
                    dependency_request.request_source.clone(),
                    dependency_request.package_alias.clone(),
                    realm_match,
                    package_id.clone(),
                );

                continue 'outer;
            }
        }

        // Look through all our packages sources in order of priority
        let (source_registry, mut candidates) = package_sources
            .source_order()
            .iter()
            .find_map(|source| {
                let registry = package_sources.get(source).unwrap();

                // Pull all of the possible candidate versions of the package we're
                // looking for from the highest priority source which has them.
                match registry.query(&dependency_request.package_req) {
                    Ok(manifests) => Some((source, manifests)),
                    Err(_) => None,
                }
            })
            .ok_or_else(|| {
                format_err!(
                    "Failed to find a source for {}",
                    dependency_request.package_req
                )
            })?;

        // Sort our candidate packages by descending version, so that we try the
        // highest versions first.
        //
        // Additionally, if there were any packages that were previously used by
        // our lockfile (in `try_to_use`), prioritize those first. This
        // technique is the one used by Cargo.
        candidates.sort_by(|a, b| {
            let contains_a = try_to_use.contains(&a.package_id());
            let contains_b = try_to_use.contains(&b.package_id());

            match (contains_a, contains_b) {
                (true, false) => Ordering::Less,
                (false, true) => Ordering::Greater,
                _ => b.package.version.cmp(&a.package.version),
            }
        });

        let filtered_candidates = candidates.iter().filter(|candidate| {
            Realm::is_dependency_valid(dependency_request.request_realm, candidate.package.realm)
        });

        let mut conflicting = Vec::new();

        for candidate in filtered_candidates {
            // Conflicts occur if two packages are SemVer compatible. We choose
            // to only allow one compatible copy of a given package to prevent
            // common user errors.

            let has_conflicting = matching_activated
                .iter()
                .any(|activated| compatible(&candidate.package.version, activated.version()));

            if has_conflicting {
                // This is a matching candidate, but it conflicts with a
                // candidate we already selected before. We'll note that this
                // happened. If there are no other matching versions that don't
                // conflict, we'll report this in an error.

                conflicting.push(candidate.package_id());
                continue;
            }

            let candidate_id = PackageId::new(
                candidate.package.name.clone(),
                candidate.package.version.clone(),
            );

            resolve.activate(
                dependency_request.request_source.clone(),
                dependency_request.package_alias.to_owned(),
                dependency_request.origin_realm,
                candidate_id.clone(),
            );

            resolve.metadata.insert(
                candidate_id.clone(),
                ResolvePackageMetadata {
                    realm: candidate.package.realm,
                    origin_realm: dependency_request.origin_realm,
                    source_registry: source_registry.clone(),
                },
            );

            for (alias, req) in &candidate.dependencies {
                packages_to_visit.push_back(DependencyRequest {
                    request_source: candidate_id.clone(),
                    request_realm: Realm::Shared,
                    origin_realm: dependency_request.origin_realm,
                    package_alias: alias.clone(),
                    package_req: req.clone(),
                })
            }

            for (alias, req) in &candidate.server_dependencies {
                packages_to_visit.push_back(DependencyRequest {
                    request_source: candidate_id.clone(),
                    request_realm: Realm::Server,
                    origin_realm: dependency_request.origin_realm,
                    package_alias: alias.clone(),
                    package_req: req.clone(),
                })
            }

            continue 'outer;
        }

        if conflicting.is_empty() {
            bail!(
                "No packages were found that matched ({req_realm:?}) {req}.\nAre you sure this is \
                 a {req_realm:?} dependency?",
                req_realm = dependency_request.request_realm,
                req = dependency_request.package_req,
            );
        } else {
            let conflicting_debug: Vec<_> = conflicting
                .into_iter()
                .map(|id| format!("{:?}", id))
                .collect();

            bail!(
                "All possible candidates for package {req} ({req_realm:?}) conflicted with other \
                 packages that were already installed. These packages were previously selected: \
                 {conflicting}",
                req = dependency_request.package_req,
                req_realm = dependency_request.request_realm,
                conflicting = conflicting_debug.join(", "),
            );
        }
    }

    Ok(resolve)
}

fn compatible(a: &Version, b: &Version) -> bool {
    if a == b {
        return true;
    }

    if a.major == 0 && b.major == 0 {
        a.minor == b.minor
    } else {
        a.major == b.major
    }
}

pub struct DependencyRequest {
    request_source: PackageId,
    request_realm: Realm,
    origin_realm: Realm,
    package_alias: String,
    package_req: PackageReq,
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::{
        package_name::PackageName, package_source::InMemoryRegistry, test_package::PackageBuilder,
    };

    fn test_project(test_name: &str, registry: InMemoryRegistry, package: PackageBuilder) -> anyhow::Result<()> {
        let package_sources = PackageSourceMap::new(Box::new(registry.source()));
        let manifest = package.into_manifest();
        let resolve = resolve(&manifest, &Default::default(), &package_sources)?;
        insta::assert_yaml_snapshot!(test_name, resolve);
        Ok(())
    }

    #[test]
    fn minimal() -> anyhow::Result<()> {
        let registry = InMemoryRegistry::new();

        let root = PackageBuilder::new("biff/minimal@0.1.0");
        test_project("minimal", registry, root)
    }

    #[test]
    fn one_dependency() -> anyhow::Result<()> {
        let registry = InMemoryRegistry::new();
        registry.publish(PackageBuilder::new("biff/minimal@0.1.0"));
        registry.publish(PackageBuilder::new("biff/minimal@0.2.0"));

        let root = PackageBuilder::new("biff/one-dependency@0.1.0")
            .with_dep("Minimal", "biff/minimal@0.1.0");
        test_project("one_dependency", registry, root)
    }

    #[test]
    fn transitive_dependency() -> anyhow::Result<()> {
        let registry = InMemoryRegistry::new();
        registry.publish(PackageBuilder::new("biff/minimal@0.1.0"));
        registry.publish(
            PackageBuilder::new("biff/one-dependency@0.1.0")
                .with_dep("Minimal", "biff/minimal@0.1.0"),
        );

        let root = PackageBuilder::new("biff/transitive-dependency@0.1.0")
            .with_dep("OneDependency", "biff/one-dependency@0.1.0");
        test_project("transitive_dependency", registry, root)
    }

    /// When there are shared dependencies, Wally should select the same
    /// dependency. Here, A depends on B and C, which both in turn depend on D.
    #[test]
    fn unified_dependencies() -> anyhow::Result<()> {
        let registry = InMemoryRegistry::new();
        registry.publish(PackageBuilder::new("biff/b@1.0.0").with_dep("D", "biff/d@1.0.0"));
        registry.publish(PackageBuilder::new("biff/c@1.0.0").with_dep("D", "biff/d@1.0.0"));
        registry.publish(PackageBuilder::new("biff/d@1.0.0"));

        let root = PackageBuilder::new("biff/a@1.0.0")
            .with_dep("B", "biff/b@1.0.0")
            .with_dep("C", "biff/c@1.0.0");

        test_project("unified_dependencies", registry, root)
    }

    /// Server dependencies are allowed to depend on shared dependencies. If a
    /// shared dependency is only depended on by server dependencies, it should
    /// be marked as server-only.
    #[test]
    fn server_to_shared() -> anyhow::Result<()> {
        let registry = InMemoryRegistry::new();
        registry.publish(PackageBuilder::new("biff/shared@1.0.0"));
        registry.publish(
            PackageBuilder::new("biff/server@1.0.0")
                .with_realm(Realm::Server)
                .with_dep("Shared", "biff/shared@1.0.0"),
        );

        let root =
            PackageBuilder::new("biff/root@1.0.0").with_server_dep("Server", "biff/server@1.0.0");

        test_project("server_to_shared", registry, root)
    }

    /// but... if that shared dependency is required by another shared dependency,
    /// (while not being also server-only) it's not server-only anymore.
    #[test]
    fn server_to_shared_and_shared_to_shared() -> anyhow::Result<()> {
        let registry = InMemoryRegistry::new();
        registry.publish(PackageBuilder::new("biff/shared@1.0.0"));
        registry.publish(
            PackageBuilder::new("biff/server@1.0.0")
                .with_realm(Realm::Server)
                .with_dep("Shared", "biff/shared@1.0.0"),
        );

        let root = PackageBuilder::new("biff/root@1.0.0")
            .with_server_dep("Server", "biff/server@1.0.0")
            .with_dep("Shared", "biff/shared@1.0.0");

        test_project("server_to_shared_and_shared_to_shared", registry, root)
    }

    /// Shared dependencies are allowed to depend on server dependencies. Server
    /// dependencies should always be marked as server-only.
    #[test]
    fn shared_to_server() -> anyhow::Result<()> {
        let registry = InMemoryRegistry::new();
        registry.publish(PackageBuilder::new("biff/server@1.0.0").with_realm(Realm::Server));

        let root =
            PackageBuilder::new("biff/root@1.0.0").with_server_dep("Server", "biff/server@1.0.0");

        test_project("shared_to_server", registry, root)
    }

    #[test]
    fn fail_server_in_shared() {
        let registry = InMemoryRegistry::new();
        registry.publish(PackageBuilder::new("biff/server@1.0.0").with_realm(Realm::Server));

        let root = PackageBuilder::new("biff/root@1.0.0").with_dep("Server", "biff/server@1.0.0");

        let package_sources = PackageSourceMap::new(Box::new(registry.source()));
        let err = resolve(root.manifest(), &Default::default(), &package_sources).unwrap_err();
        insta::assert_snapshot!(err);
    }

    /// Tests the simple one dependency case, except that a new version of the
    /// dependency will be published after the initial resolve. By persisting
    /// the set of activated packages from the initial install, we signal that
    /// the dependency should not be upgraded.
    #[test]
    fn one_dependency_no_upgrade() -> anyhow::Result<()> {
        let registry = InMemoryRegistry::new();
        registry.publish(PackageBuilder::new("biff/minimal@1.0.0"));

        let root = PackageBuilder::new("biff/one-dependency@1.0.0")
            .with_dep("Minimal", "biff/minimal@1.0.0");

        let package_sources = PackageSourceMap::new(Box::new(registry.source()));

        let resolved = resolve(root.manifest(), &Default::default(), &package_sources)?;
        insta::assert_yaml_snapshot!("one_dependency_no_upgrade", resolved);

        registry.publish(PackageBuilder::new("biff/minimal@1.1.0"));
        let new_resolved = resolve(root.manifest(), &resolved.activated, &package_sources)?;
        insta::assert_yaml_snapshot!("one_dependency_no_upgrade", new_resolved);

        Ok(())
    }

    #[test]
    fn one_dependency_yes_upgrade() -> anyhow::Result<()> {
        let registry = InMemoryRegistry::new();
        registry.publish(PackageBuilder::new("biff/minimal@1.0.0"));

        let root = PackageBuilder::new("biff/one-dependency@1.0.0")
            .with_dep("Minimal", "biff/minimal@1.0.0");

        let package_sources = PackageSourceMap::new(Box::new(registry.source()));

        let resolved = resolve(root.manifest(), &Default::default(), &package_sources)?;
        insta::assert_yaml_snapshot!(resolved);

        // We can indicate that we'd like to upgrade a package by just removing
        // it from the try_to_use set!
        let remove_this: PackageName = "biff/minimal".parse().unwrap();
        let try_to_use = resolved
            .activated
            .into_iter()
            .filter(|id| id.name() != &remove_this)
            .collect();

        registry.publish(PackageBuilder::new("biff/minimal@1.1.0"));
        let new_resolved = resolve(root.manifest(), &try_to_use, &package_sources)?;
        insta::assert_yaml_snapshot!(new_resolved);

        Ok(())
    }
}
