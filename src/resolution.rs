use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::path::PathBuf;

use anyhow::bail;
use anyhow::format_err;
use semver::Version;
use serde::Serialize;

use crate::manifest::{Manifest, Realm};
use crate::package_id::PackageId;
use crate::package_origin::PackageOrigin;
use crate::package_req::PackageReq;
use crate::package_source::{PackageSourceId, PackageSourceMap};

/// A completely resolved graph of packages returned by `resolve`.
///
/// State here is stored in multiple maps, all keyed by PackageId, to facilitate
/// concurrent mutable access to unrelated information about different packages.
#[derive(Debug, Default, Serialize)]
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

    pub fn get_try_to_use(&self) -> BTreeMap<PackageId, PackageOrigin> {
        let mut try_to_use = BTreeMap::new();

        for (package_id, package_metadata) in self.metadata.iter() {
            try_to_use.insert(package_id.clone(), package_metadata.package_origin.clone());
        }

        try_to_use
    }
}

/// A single node in the package resolution graph.
#[derive(Debug, Serialize)]
pub struct ResolvePackageMetadata {
    pub realm: Realm,
    pub origin_realm: Realm,
    pub package_origin: PackageOrigin,
}

pub struct DependencyRequest {
    request_source: PackageId,
    request_realm: Realm,
    origin_realm: Realm,
    package_alias: String,
    package_req: PackageReq,
    package_origin: PackageOrigin,
}

pub fn resolve(
    root_manifest: &Manifest,
    root_dir: &PathBuf,
    try_to_use: &BTreeMap<PackageId, PackageOrigin>,
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
            package_origin: PackageOrigin::Registry(PackageSourceId::DefaultRegistry),
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
            package_req: req.package_req(root_dir)?,
            package_origin: req.package_origin(),
        });
    }

    for (alias, req) in &root_manifest.server_dependencies {
        packages_to_visit.push_back(DependencyRequest {
            request_source: root_manifest.package_id(),
            request_realm: Realm::Server,
            origin_realm: Realm::Server,
            package_alias: alias.clone(),
            package_req: req.package_req(root_dir)?,
            package_origin: req.package_origin(),
        });
    }

    for (alias, req) in &root_manifest.dev_dependencies {
        packages_to_visit.push_back(DependencyRequest {
            request_source: root_manifest.package_id(),
            request_realm: Realm::Dev,
            origin_realm: Realm::Dev,
            package_alias: alias.clone(),
            package_req: req.package_req(root_dir)?,
            package_origin: req.package_origin(),
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
                resolve.activate(
                    dependency_request.request_source.clone(),
                    dependency_request.package_alias.clone(),
                    dependency_request.request_realm,
                    package_id.clone(),
                );

                let metadata = resolve
                    .metadata
                    .get_mut(package_id)
                    .expect("activated package was missing metadata");

                // We want to set the origin to the least restrictive origin possible.
                // For example we want to keep packages in the dev realm unless a dependency
                // with a shared/server origin requires it. This way server/shared dependencies
                // which only originate from dev dependencies get put into the dev folder even
                // if they usually belong to another realm.
                metadata.origin_realm =
                    match (metadata.origin_realm, dependency_request.origin_realm) {
                        (_, Realm::Shared) => Realm::Shared,
                        (Realm::Shared, _) => Realm::Shared,
                        (_, Realm::Server) => Realm::Server,
                        (Realm::Server, _) => Realm::Server,
                        (Realm::Dev, Realm::Dev) => Realm::Dev,
                    };

                continue 'outer;
            }
        }

        // Based on the origin of the dependency request, let's pull in the possible candidates.
        let (package_origin, mut candidates) = match dependency_request.package_origin {
            PackageOrigin::Path(path) => {
                // It's illegal for any sub-packages to have path dependencies.
                if dependency_request.request_source != root_manifest.package_id() {
                    bail!(format!(
                        "Unexpected path dependency ({}) within the {} dependency.",
                        path.display(),
                        dependency_request.request_source
                    ))
                }

                let candidate = Manifest::load(&root_dir.join(&path))?;
                // TODO: Some way to convert source_registry into a PackageSourceId?
                // That way, it can be used later on for the dependencies of this candidate(?)
                let _source_registry = candidate.package.registry.clone();
                let manifests = vec![candidate];

                (PackageOrigin::Path(path), manifests)
            }
            PackageOrigin::Git(_) => todo!(),
            // TODO: Worth investigating with what we can do if we're given the originating source.
            PackageOrigin::Registry(_) => {
                // Look through all our packages sources in order of priority
                package_sources
                    .source_order()
                    .iter()
                    .find_map(|source| {
                        let registry = package_sources.get(source).unwrap();

                        // Pull all of the possible candidate versions of the package we're
                        // looking for from the highest priority source which has them.
                        match registry.query(&dependency_request.package_req) {
                            Ok(manifests) => {
                                Some((PackageOrigin::Registry(source.clone()), manifests))
                            }
                            Err(_) => None,
                        }
                    })
                    .ok_or_else(|| {
                        format_err!(
                            "Failed to find a source for {}",
                            dependency_request.package_req
                        )
                    })?
            }
        };

        // Sort our candidate packages by descending version, so that we try the
        // highest versions first.
        //
        // Additionally, if there were any packages that were previously used by
        // our lockfile (in `try_to_use`), prioritize those first. This
        // technique is the one used by Cargo.
        candidates.sort_by(|a, b| {
            let contains_a = try_to_use.contains_key(&a.package_id());
            let contains_b = try_to_use.contains_key(&b.package_id());

            match (contains_a, contains_b) {
                (true, false) => Ordering::Less,
                (false, true) => Ordering::Greater,
                _ => b.package.version.cmp(&a.package.version),
            }
        });

        let request_realm = dependency_request.request_realm;

        let filtered_candidates = candidates
            .iter()
            .filter(|candidate| Realm::is_dependency_valid(request_realm, candidate.package.realm));

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
                dependency_request.request_realm,
                candidate_id.clone(),
            );

            resolve.metadata.insert(
                candidate_id.clone(),
                ResolvePackageMetadata {
                    realm: candidate.package.realm,
                    origin_realm: dependency_request.origin_realm,
                    package_origin,
                },
            );

            for (alias, req) in &candidate.dependencies {
                packages_to_visit.push_back(DependencyRequest {
                    request_source: candidate_id.clone(),
                    request_realm: Realm::Shared,
                    origin_realm: dependency_request.origin_realm,
                    package_alias: alias.clone(),
                    package_req: req.package_req(root_dir)?,
                    // TODO: what happens if a package dependency also has a path dependency?
                    // I don't think Wally is going to resolve it correctly...
                    // It shouldn't be possible because a dependency shouldn't have a path dependency.
                    package_origin: req.package_origin(),
                })
            }

            for (alias, req) in &candidate.server_dependencies {
                packages_to_visit.push_back(DependencyRequest {
                    request_source: candidate_id.clone(),
                    request_realm: Realm::Server,
                    origin_realm: dependency_request.origin_realm,
                    package_alias: alias.clone(),
                    package_req: req.package_req(root_dir)?,
                    // Same for the loop previously.
                    package_origin: req.package_origin(),
                })
            }

            continue 'outer;
        }

        if conflicting.is_empty() {
            bail!(
                "No packages were found that matched ({req_realm:?}) {req}.\
                \nAre you sure this is a {req_realm:?} dependency?",
                req_realm = dependency_request.request_realm,
                req = dependency_request.package_req,
            );
        } else {
            let conflicting_debug: Vec<_> = conflicting
                .into_iter()
                .map(|id| format!("{:?}", id))
                .collect();

            bail!(
                "All possible candidates for package {req} ({req_realm:?}) \
                 conflicted with other packages that were already installed. \
                 These packages were previously selected: {conflicting}",
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

#[cfg(test)]
mod tests {
    use super::*;

    use crate::{
        package_name::PackageName, package_source::InMemoryRegistry, test_package::PackageBuilder,
    };

    fn test_project(registry: InMemoryRegistry, package: PackageBuilder) -> anyhow::Result<()> {
        let package_sources = PackageSourceMap::new(Box::new(registry.source()));
        let manifest = package.into_manifest();
        let resolve = resolve(
            &manifest,
            &Default::default(),
            &Default::default(),
            &package_sources,
        )?;
        insta::assert_yaml_snapshot!(resolve);
        Ok(())
    }

    #[test]
    fn minimal() -> anyhow::Result<()> {
        let registry = InMemoryRegistry::new();

        let root = PackageBuilder::new("biff/minimal@0.1.0");
        test_project(registry, root)
    }

    #[test]
    fn one_dependency() -> anyhow::Result<()> {
        let registry = InMemoryRegistry::new();
        registry.publish(PackageBuilder::new("biff/minimal@0.1.0"));
        registry.publish(PackageBuilder::new("biff/minimal@0.2.0"));

        let root = PackageBuilder::new("biff/one-dependency@0.1.0")
            .with_dep("Minimal", "biff/minimal@0.1.0");
        test_project(registry, root)
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
        test_project(registry, root)
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

        test_project(registry, root)
    }

    /// Server dependencies are allowed to depend on shared dependencies. If a
    /// shared dependency is only depended on by server dependencies, it should
    /// be marked as server-only.
    #[test]
    fn server_to_shared() -> anyhow::Result<()> {
        let registry = InMemoryRegistry::new();
        registry.publish(PackageBuilder::new("biff/shared@1.0.0"));

        let root =
            PackageBuilder::new("biff/root@1.0.0").with_server_dep("Shared", "biff/shared@1.0.0");

        test_project(registry, root)
    }

    /// Shared dependencies are allowed to depend on server dependencies. Server
    /// dependencies should always be marked as server-only.
    #[test]
    fn shared_to_server() -> anyhow::Result<()> {
        let registry = InMemoryRegistry::new();
        registry.publish(PackageBuilder::new("biff/server@1.0.0").with_realm(Realm::Server));

        let root =
            PackageBuilder::new("biff/root@1.0.0").with_server_dep("Server", "biff/server@1.0.0");

        test_project(registry, root)
    }

    #[test]
    fn fail_server_in_shared() {
        let registry = InMemoryRegistry::new();
        registry.publish(PackageBuilder::new("biff/server@1.0.0").with_realm(Realm::Server));

        let root = PackageBuilder::new("biff/root@1.0.0").with_dep("Server", "biff/server@1.0.0");

        let package_sources = PackageSourceMap::new(Box::new(registry.source()));
        let err = resolve(
            root.manifest(),
            &Default::default(),
            &Default::default(),
            &package_sources,
        )
        .unwrap_err();
        insta::assert_display_snapshot!(err);
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

        let resolved = resolve(
            root.manifest(),
            &Default::default(),
            &Default::default(),
            &package_sources,
        )?;
        insta::assert_yaml_snapshot!("one_dependency_no_upgrade", resolved);

        registry.publish(PackageBuilder::new("biff/minimal@1.1.0"));
        let new_resolved = resolve(
            root.manifest(),
            &Default::default(),
            &resolved.get_try_to_use(),
            &package_sources,
        )?;
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

        let resolved = resolve(
            root.manifest(),
            &Default::default(),
            &Default::default(),
            &package_sources,
        )?;
        insta::assert_yaml_snapshot!(resolved);

        // We can indicate that we'd like to upgrade a package by just removing
        // it from the try_to_use set!
        let remove_this: PackageName = "biff/minimal".parse().unwrap();
        let try_to_use = resolved
            .get_try_to_use()
            .into_iter()
            .filter(|(id, _)| id.name() != &remove_this)
            .collect();

        registry.publish(PackageBuilder::new("biff/minimal@1.1.0"));
        let new_resolved = resolve(
            root.manifest(),
            &Default::default(),
            &try_to_use,
            &package_sources,
        )?;
        insta::assert_yaml_snapshot!(new_resolved);

        Ok(())
    }
}
