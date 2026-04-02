use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::path::{Path, PathBuf};

use anyhow::{bail, format_err};
use semver::Version;
use serde::Serialize;

use crate::dependency_spec::DependencySpec;
use crate::manifest::{Manifest, Realm};
use crate::package_id::PackageId;
use crate::package_req::PackageReq;
use crate::package_source::{PackageSourceId, PackageSourceMap, PackageSourceProvider};
use crate::workspace::Workspace;

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
    /// `true` when this package is a local workspace member (activated via
    /// [`resolve_workspace`]) rather than a package fetched from a registry.
    pub is_workspace_member: bool,
}

/// Resolve dependencies for a single-package project.
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
            is_workspace_member: false,
        },
    );

    // Queue of all dependency requests that need to be resolved.
    let mut packages_to_visit = VecDeque::new();

    enqueue_manifest_deps(
        root_manifest,
        root_manifest.package_id(),
        None,
        &mut packages_to_visit,
    );

    resolve_queued(
        &mut resolve,
        &mut packages_to_visit,
        try_to_use,
        package_sources,
        None,
    )?;

    Ok(resolve)
}

/// Resolve dependencies for a multi-member workspace.
///
/// All workspace members are activated as roots with
/// [`PackageSourceId::Path`]. Path dependencies between members are resolved
/// eagerly by looking up the target in the workspace member set. Registry
/// dependencies follow the standard resolution path.
pub fn resolve_workspace(
    workspace: &Workspace,
    try_to_use: &BTreeSet<PackageId>,
    package_sources: &PackageSourceMap,
) -> anyhow::Result<Resolve> {
    let mut resolve = Resolve::default();
    let mut packages_to_visit = VecDeque::new();

    let is_single_package = workspace.is_single_package();

    for (member_dir, manifest) in workspace.members() {
        let member_id = manifest.package_id();
        resolve.activated.insert(member_id.clone());

        // Single-package projects are treated identically to the legacy
        // `resolve()` path: the root is not a workspace member.
        let (source_registry, is_workspace_member) = if is_single_package {
            (PackageSourceId::DefaultRegistry, false)
        } else {
            (PackageSourceId::Path(member_dir.clone()), true)
        };

        resolve.metadata.insert(
            member_id.clone(),
            ResolvePackageMetadata {
                realm: manifest.package.realm,
                origin_realm: manifest.package.realm,
                source_registry,
                is_workspace_member,
            },
        );

        enqueue_manifest_deps(
            manifest,
            member_id,
            Some(member_dir.as_path()),
            &mut packages_to_visit,
        );
    }

    resolve_queued(
        &mut resolve,
        &mut packages_to_visit,
        try_to_use,
        package_sources,
        Some(workspace),
    )?;

    Ok(resolve)
}

/// Core BFS resolution loop shared by [`resolve`] and [`resolve_workspace`].
fn resolve_queued(
    resolve: &mut Resolve,
    packages_to_visit: &mut VecDeque<DependencyRequest>,
    try_to_use: &BTreeSet<PackageId>,
    package_sources: &PackageSourceMap,
    workspace: Option<&Workspace>,
) -> anyhow::Result<()> {
    // Workhorse loop: resolve all dependencies, breadth-first.
    'outer: while let Some(dependency_request) = packages_to_visit.pop_front() {
        // -------------------------------------------------------------
        // Path dependencies: resolve eagerly against the workspace
        // -------------------------------------------------------------
        if let DependencyRequestKind::Path(ref target_dir) = dependency_request.kind {
            let ws = workspace.expect(
                "path dependency encountered but no workspace context is available",
            );

            let target_manifest = ws.get_member_at_path(target_dir).ok_or_else(|| {
                format_err!(
                    "path dependency '{}' at {} does not point to a workspace member",
                    dependency_request.package_alias,
                    target_dir.display()
                )
            })?;

            if !Realm::is_dependency_valid(
                dependency_request.request_realm,
                target_manifest.package.realm,
            ) {
                bail!(
                    "Package {} has a {:?} dependency '{}' on {}, \
                     but {} has realm {:?} which is not compatible",
                    dependency_request.request_source,
                    dependency_request.request_realm,
                    dependency_request.package_alias,
                    target_manifest.package_id(),
                    target_manifest.package_id(),
                    target_manifest.package.realm,
                );
            }

            let target_id = target_manifest.package_id();

            // All workspace members were activated as roots, so the target
            // must already be in the metadata map.
            let metadata = resolve.metadata.get_mut(&target_id).unwrap_or_else(|| {
                panic!(
                    "workspace member {} should have been activated as a root",
                    target_id
                )
            });

            let merged =
                merge_origin_realm(metadata.origin_realm, dependency_request.origin_realm);
            metadata.origin_realm = merged;

            resolve.activate(
                dependency_request.request_source.clone(),
                dependency_request.package_alias.clone(),
                merged,
                target_id,
            );

            continue 'outer;
        }

        // -------------------------------------------------------------
        // Registry dependencies
        // -------------------------------------------------------------
        let package_req = match &dependency_request.kind {
            DependencyRequestKind::Registry(req) => req,
            _ => unreachable!("non-registry, non-path request in resolve loop"),
        };

        // Locate all already-activated packages that might match this
        // dependency request.
        let mut matching_activated: Vec<_> = resolve
            .activated
            .iter()
            .filter(|package_id| package_id.name() == package_req.name())
            .cloned()
            .collect();

        // Sort our list of candidates by descending version so that we can pick
        // newest candidates first.
        matching_activated.sort_by(|a, b| b.version().cmp(a.version()));

        // Check for the highest version already-activated package that matches
        // our constraints.
        for package_id in &matching_activated {
            if package_req.matches_id(package_id) {
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
                let realm_match =
                    merge_origin_realm(metadata.origin_realm, dependency_request.origin_realm);

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
                match registry.query(package_req) {
                    Ok(manifests) => Some((source, manifests)),
                    Err(_) => None,
                }
            })
            .ok_or_else(|| {
                format_err!(
                    "Failed to find a source for {}",
                    package_req
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
                    is_workspace_member: false,
                },
            );

            enqueue_transitive_deps(
                candidate,
                candidate_id.clone(),
                dependency_request.origin_realm,
                None,
                packages_to_visit,
            );

            continue 'outer;
        }

        if conflicting.is_empty() {
            bail!(
                "No packages were found that matched ({req_realm:?}) {req}.\nAre you sure this is \
                 a {req_realm:?} dependency?",
                req_realm = dependency_request.request_realm,
                req = package_req,
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
                req = package_req,
                req_realm = dependency_request.request_realm,
                conflicting = conflicting_debug.join(", "),
            );
        }
    }

    Ok(())
}

/// Merge two origin realms, selecting the least restrictive (most visible).
/// Shared is the least restrictive, Dev is the most restrictive.
fn merge_origin_realm(existing: Realm, incoming: Realm) -> Realm {
    match (existing, incoming) {
        (_, Realm::Shared) | (Realm::Shared, _) => Realm::Shared,
        (_, Realm::Server) | (Realm::Server, _) => Realm::Server,
        (Realm::Dev, Realm::Dev) => Realm::Dev,
    }
}

/// Convert a [`DependencySpec`] into a [`DependencyRequestKind`].
///
/// `source_dir` is required when path dependencies may be present (i.e. in a
/// workspace context). It is used to resolve relative paths.
fn dep_spec_to_request_kind(
    alias: &str,
    spec: &DependencySpec,
    source_dir: Option<&Path>,
) -> DependencyRequestKind {
    match spec {
        DependencySpec::Registry(req) => DependencyRequestKind::Registry(req.clone()),
        DependencySpec::Path(path_spec) => {
            let dir = source_dir.unwrap_or_else(|| {
                panic!(
                    "path dependency '{}' encountered without source directory context",
                    alias
                )
            });
            DependencyRequestKind::Path(dir.join(&path_spec.path))
        }
        DependencySpec::Workspace { .. } => {
            panic!(
                "dependency '{}' is DependencySpec::Workspace which should have been \
                 resolved during workspace loading",
                alias
            );
        }
    }
}

/// Enqueue all dependencies from a root manifest (shared, server, and dev).
fn enqueue_manifest_deps(
    manifest: &Manifest,
    source_id: PackageId,
    source_dir: Option<&Path>,
    queue: &mut VecDeque<DependencyRequest>,
) {
    for (alias, spec) in &manifest.dependencies {
        queue.push_back(DependencyRequest {
            request_source: source_id.clone(),
            request_realm: Realm::Shared,
            origin_realm: Realm::Shared,
            package_alias: alias.clone(),
            kind: dep_spec_to_request_kind(alias, spec, source_dir),
        });
    }

    for (alias, spec) in &manifest.server_dependencies {
        queue.push_back(DependencyRequest {
            request_source: source_id.clone(),
            request_realm: Realm::Server,
            origin_realm: Realm::Server,
            package_alias: alias.clone(),
            kind: dep_spec_to_request_kind(alias, spec, source_dir),
        });
    }

    for (alias, spec) in &manifest.dev_dependencies {
        queue.push_back(DependencyRequest {
            request_source: source_id.clone(),
            request_realm: Realm::Dev,
            origin_realm: Realm::Dev,
            package_alias: alias.clone(),
            kind: dep_spec_to_request_kind(alias, spec, source_dir),
        });
    }
}

/// Enqueue transitive dependencies (shared + server only, not dev).
fn enqueue_transitive_deps(
    manifest: &Manifest,
    source_id: PackageId,
    origin_realm: Realm,
    source_dir: Option<&Path>,
    queue: &mut VecDeque<DependencyRequest>,
) {
    for (alias, spec) in &manifest.dependencies {
        queue.push_back(DependencyRequest {
            request_source: source_id.clone(),
            request_realm: Realm::Shared,
            origin_realm,
            package_alias: alias.clone(),
            kind: dep_spec_to_request_kind(alias, spec, source_dir),
        });
    }

    for (alias, spec) in &manifest.server_dependencies {
        queue.push_back(DependencyRequest {
            request_source: source_id.clone(),
            request_realm: Realm::Server,
            origin_realm,
            package_alias: alias.clone(),
            kind: dep_spec_to_request_kind(alias, spec, source_dir),
        });
    }
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

enum DependencyRequestKind {
    Registry(PackageReq),
    Path(PathBuf),
}

struct DependencyRequest {
    request_source: PackageId,
    request_realm: Realm,
    origin_realm: Realm,
    package_alias: String,
    kind: DependencyRequestKind,
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
        let resolve = resolve(&manifest, &Default::default(), &package_sources)?;
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
        registry.publish(
            PackageBuilder::new("biff/server@1.0.0")
                .with_realm(Realm::Server)
                .with_dep("Shared", "biff/shared@1.0.0"),
        );

        let root =
            PackageBuilder::new("biff/root@1.0.0").with_server_dep("Server", "biff/server@1.0.0");

        test_project(registry, root)
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
        let err = resolve(root.manifest(), &Default::default(), &package_sources).unwrap_err();
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

    // =====================================================================
    // Workspace resolution tests
    // =====================================================================

    mod workspace_tests {
        use super::*;
        use std::fs;
        use tempfile::TempDir;

        fn write_manifest(dir: &std::path::Path, content: &str) {
            fs::create_dir_all(dir).unwrap();
            fs::write(dir.join("wally.toml"), content).unwrap();
        }

        fn assert_activated(resolve: &Resolve, name: &str) {
            let pkg_name: PackageName = name.parse().unwrap();
            assert!(
                resolve
                    .activated
                    .iter()
                    .any(|id| id.name() == &pkg_name),
                "expected {} to be activated, activated: {:?}",
                name,
                resolve.activated
            );
        }

        fn find_package_id(resolve: &Resolve, name: &str) -> PackageId {
            let pkg_name: PackageName = name.parse().unwrap();
            resolve
                .activated
                .iter()
                .find(|id| id.name() == &pkg_name)
                .cloned()
                .unwrap_or_else(|| panic!("package {} not found in activated set", name))
        }

        /// Two members, no cross-deps -- basic workspace resolution.
        #[test]
        fn workspace_two_members_no_cross_deps() -> anyhow::Result<()> {
            let tmp = TempDir::new()?;
            write_manifest(
                tmp.path(),
                r#"
                [workspace]
                members = ["modules/*"]
                registry = "https://example.com/index"
                realm = "server"
                "#,
            );
            write_manifest(
                &tmp.path().join("modules/alpha"),
                r#"
                [package]
                name = "team/alpha"
                version = "0.1.0"
                registry = "https://example.com/index"
                realm = "server"
                "#,
            );
            write_manifest(
                &tmp.path().join("modules/beta"),
                r#"
                [package]
                name = "team/beta"
                version = "0.2.0"
                registry = "https://example.com/index"
                realm = "server"
                "#,
            );

            let workspace = Workspace::load(tmp.path())?;
            let registry = InMemoryRegistry::new();
            let sources = PackageSourceMap::new(Box::new(registry.source()));
            let resolved = resolve_workspace(&workspace, &Default::default(), &sources)?;

            assert_eq!(resolved.activated.len(), 2);
            assert_activated(&resolved, "team/alpha");
            assert_activated(&resolved, "team/beta");

            let alpha_id = find_package_id(&resolved, "team/alpha");
            let beta_id = find_package_id(&resolved, "team/beta");

            assert!(matches!(
                resolved.metadata[&alpha_id].source_registry,
                PackageSourceId::Path(_)
            ));
            assert!(matches!(
                resolved.metadata[&beta_id].source_registry,
                PackageSourceId::Path(_)
            ));

            Ok(())
        }

        /// A depends on B via path -- path dep creates correct edge.
        #[test]
        fn workspace_path_dep_a_to_b() -> anyhow::Result<()> {
            let tmp = TempDir::new()?;
            write_manifest(
                tmp.path(),
                r#"
                [workspace]
                members = ["modules/*"]
                registry = "https://example.com/index"
                realm = "shared"
                "#,
            );
            write_manifest(
                &tmp.path().join("modules/foo"),
                r#"
                [package]
                name = "team/foo"
                version = "1.0.0"
                registry = "https://example.com/index"
                realm = "shared"

                [dependencies]
                Bar = { path = "../bar" }
                "#,
            );
            write_manifest(
                &tmp.path().join("modules/bar"),
                r#"
                [package]
                name = "team/bar"
                version = "1.0.0"
                registry = "https://example.com/index"
                realm = "shared"
                "#,
            );

            let workspace = Workspace::load(tmp.path())?;
            let registry = InMemoryRegistry::new();
            let sources = PackageSourceMap::new(Box::new(registry.source()));
            let resolved = resolve_workspace(&workspace, &Default::default(), &sources)?;

            assert_eq!(resolved.activated.len(), 2);

            let foo_id = find_package_id(&resolved, "team/foo");
            let bar_id = find_package_id(&resolved, "team/bar");

            let foo_shared_deps = &resolved.shared_dependencies[&foo_id];
            assert_eq!(foo_shared_deps.get("Bar"), Some(&bar_id));

            Ok(())
        }

        /// Diamond: A -> B (path) + A -> C (registry), B -> C (registry).
        /// C should be unified (activated once).
        #[test]
        fn workspace_diamond_path_and_registry() -> anyhow::Result<()> {
            let tmp = TempDir::new()?;
            write_manifest(
                tmp.path(),
                r#"
                [workspace]
                members = ["modules/*"]
                registry = "https://example.com/index"
                realm = "shared"
                "#,
            );
            write_manifest(
                &tmp.path().join("modules/app"),
                r#"
                [package]
                name = "team/app"
                version = "1.0.0"
                registry = "https://example.com/index"
                realm = "shared"

                [dependencies]
                Lib = { path = "../lib" }
                Util = "ext/util@1.0.0"
                "#,
            );
            write_manifest(
                &tmp.path().join("modules/lib"),
                r#"
                [package]
                name = "team/lib"
                version = "1.0.0"
                registry = "https://example.com/index"
                realm = "shared"

                [dependencies]
                Util = "ext/util@1.0.0"
                "#,
            );

            let workspace = Workspace::load(tmp.path())?;
            let registry = InMemoryRegistry::new();
            registry.publish(PackageBuilder::new("ext/util@1.0.0"));
            let sources = PackageSourceMap::new(Box::new(registry.source()));
            let resolved = resolve_workspace(&workspace, &Default::default(), &sources)?;

            // 2 workspace members + 1 registry package = 3
            assert_eq!(resolved.activated.len(), 3);

            let app_id = find_package_id(&resolved, "team/app");
            let lib_id = find_package_id(&resolved, "team/lib");
            let util_id = find_package_id(&resolved, "ext/util");

            // app -> lib (path), app -> util (registry)
            let app_deps = &resolved.shared_dependencies[&app_id];
            assert_eq!(app_deps.get("Lib"), Some(&lib_id));
            assert_eq!(app_deps.get("Util"), Some(&util_id));

            // lib -> util (registry)
            let lib_deps = &resolved.shared_dependencies[&lib_id];
            assert_eq!(lib_deps.get("Util"), Some(&util_id));

            // util is a registry package
            assert_eq!(
                resolved.metadata[&util_id].source_registry,
                PackageSourceId::DefaultRegistry
            );

            Ok(())
        }

        /// Path dep to non-member directory should error.
        #[test]
        fn workspace_path_dep_non_member_errors() {
            let tmp = TempDir::new().unwrap();
            write_manifest(
                tmp.path(),
                r#"
                [workspace]
                members = ["modules/*"]
                registry = "https://example.com/index"
                realm = "shared"
                "#,
            );
            write_manifest(
                &tmp.path().join("modules/foo"),
                r#"
                [package]
                name = "team/foo"
                version = "1.0.0"
                registry = "https://example.com/index"
                realm = "shared"

                [dependencies]
                External = { path = "../../nonexistent" }
                "#,
            );

            let workspace = Workspace::load(tmp.path()).unwrap();
            let registry = InMemoryRegistry::new();
            let sources = PackageSourceMap::new(Box::new(registry.source()));
            let result = resolve_workspace(&workspace, &Default::default(), &sources);
            assert!(result.is_err());
            let err = result.unwrap_err().to_string();
            assert!(
                err.contains("External") || err.contains("workspace member"),
                "error should mention the dependency name or workspace: {}",
                err
            );
        }

        /// Shared member cannot depend on server member via [dependencies].
        #[test]
        fn workspace_realm_validation_shared_cannot_dep_on_server() {
            let tmp = TempDir::new().unwrap();
            write_manifest(
                tmp.path(),
                r#"
                [workspace]
                members = ["modules/*"]
                registry = "https://example.com/index"
                "#,
            );
            write_manifest(
                &tmp.path().join("modules/client"),
                r#"
                [package]
                name = "team/client"
                version = "1.0.0"
                registry = "https://example.com/index"
                realm = "shared"

                [dependencies]
                ServerLib = { path = "../server-lib" }
                "#,
            );
            write_manifest(
                &tmp.path().join("modules/server-lib"),
                r#"
                [package]
                name = "team/server-lib"
                version = "1.0.0"
                registry = "https://example.com/index"
                realm = "server"
                "#,
            );

            let workspace = Workspace::load(tmp.path()).unwrap();
            let registry = InMemoryRegistry::new();
            let sources = PackageSourceMap::new(Box::new(registry.source()));
            let result = resolve_workspace(&workspace, &Default::default(), &sources);
            assert!(result.is_err());
            let err = result.unwrap_err().to_string();
            assert!(
                err.contains("realm") || err.contains("compatible"),
                "error should mention realm incompatibility: {}",
                err
            );
        }

        /// Server member CAN depend on shared member (server deps can use shared).
        /// Because shared-lib is pre-activated as a root with origin_realm=Shared,
        /// the merged origin stays Shared and the edge is recorded in shared_dependencies.
        #[test]
        fn workspace_server_member_depends_on_shared_via_path() -> anyhow::Result<()> {
            let tmp = TempDir::new()?;
            write_manifest(
                tmp.path(),
                r#"
                [workspace]
                members = ["modules/*"]
                registry = "https://example.com/index"
                "#,
            );
            write_manifest(
                &tmp.path().join("modules/svc"),
                r#"
                [package]
                name = "team/svc"
                version = "1.0.0"
                registry = "https://example.com/index"
                realm = "server"

                [server-dependencies]
                SharedLib = { path = "../shared-lib" }
                "#,
            );
            write_manifest(
                &tmp.path().join("modules/shared-lib"),
                r#"
                [package]
                name = "team/shared-lib"
                version = "1.0.0"
                registry = "https://example.com/index"
                realm = "shared"
                "#,
            );

            let workspace = Workspace::load(tmp.path())?;
            let registry = InMemoryRegistry::new();
            let sources = PackageSourceMap::new(Box::new(registry.source()));
            let resolved = resolve_workspace(&workspace, &Default::default(), &sources)?;

            let svc_id = find_package_id(&resolved, "team/svc");
            let shared_lib_id = find_package_id(&resolved, "team/shared-lib");

            // shared-lib is pre-activated with origin_realm=Shared, so the
            // merged origin is Shared and the edge lands in shared_dependencies.
            let svc_shared_deps = &resolved.shared_dependencies[&svc_id];
            assert_eq!(svc_shared_deps.get("SharedLib"), Some(&shared_lib_id));

            assert_eq!(
                resolved.metadata[&shared_lib_id].origin_realm,
                Realm::Shared
            );

            Ok(())
        }

        /// Origin realm merging: a shared member initially activated as Shared
        /// stays Shared even when also depended upon from a server context.
        #[test]
        fn workspace_origin_realm_merging() -> anyhow::Result<()> {
            let tmp = TempDir::new()?;
            write_manifest(
                tmp.path(),
                r#"
                [workspace]
                members = ["modules/*"]
                registry = "https://example.com/index"
                "#,
            );
            write_manifest(
                &tmp.path().join("modules/shared-user"),
                r#"
                [package]
                name = "team/shared-user"
                version = "1.0.0"
                registry = "https://example.com/index"
                realm = "shared"

                [dependencies]
                Common = { path = "../common" }
                "#,
            );
            write_manifest(
                &tmp.path().join("modules/server-user"),
                r#"
                [package]
                name = "team/server-user"
                version = "1.0.0"
                registry = "https://example.com/index"
                realm = "server"

                [server-dependencies]
                Common = { path = "../common" }
                "#,
            );
            write_manifest(
                &tmp.path().join("modules/common"),
                r#"
                [package]
                name = "team/common"
                version = "1.0.0"
                registry = "https://example.com/index"
                realm = "shared"
                "#,
            );

            let workspace = Workspace::load(tmp.path())?;
            let registry = InMemoryRegistry::new();
            let sources = PackageSourceMap::new(Box::new(registry.source()));
            let resolved = resolve_workspace(&workspace, &Default::default(), &sources)?;

            let common_id = find_package_id(&resolved, "team/common");
            // Common is depended on from both shared and server contexts.
            // The merge should promote to Shared (least restrictive).
            assert_eq!(resolved.metadata[&common_id].origin_realm, Realm::Shared);

            Ok(())
        }

        /// Dev deps are resolved for member roots.
        #[test]
        fn workspace_dev_deps_resolved_for_members() -> anyhow::Result<()> {
            let tmp = TempDir::new()?;
            write_manifest(
                tmp.path(),
                r#"
                [workspace]
                members = ["modules/*"]
                registry = "https://example.com/index"
                realm = "shared"
                "#,
            );
            write_manifest(
                &tmp.path().join("modules/lib"),
                r#"
                [package]
                name = "team/lib"
                version = "1.0.0"
                registry = "https://example.com/index"
                realm = "shared"

                [dev-dependencies]
                TestEZ = "roblox/testez@0.4.0"
                "#,
            );

            let workspace = Workspace::load(tmp.path())?;
            let registry = InMemoryRegistry::new();
            registry.publish(PackageBuilder::new("roblox/testez@0.4.0"));
            let sources = PackageSourceMap::new(Box::new(registry.source()));
            let resolved = resolve_workspace(&workspace, &Default::default(), &sources)?;

            // lib + testez
            assert_eq!(resolved.activated.len(), 2);

            let lib_id = find_package_id(&resolved, "team/lib");
            let testez_id = find_package_id(&resolved, "roblox/testez");

            let lib_dev_deps = &resolved.dev_dependencies[&lib_id];
            assert_eq!(lib_dev_deps.get("TestEZ"), Some(&testez_id));

            // TestEZ should have origin_realm = Dev (only used via dev deps)
            assert_eq!(resolved.metadata[&testez_id].origin_realm, Realm::Dev);

            Ok(())
        }

        /// Lockfile try_to_use bias works in workspace context.
        #[test]
        fn workspace_try_to_use_bias() -> anyhow::Result<()> {
            let tmp = TempDir::new()?;
            write_manifest(
                tmp.path(),
                r#"
                [workspace]
                members = ["modules/*"]
                registry = "https://example.com/index"
                realm = "shared"
                "#,
            );
            write_manifest(
                &tmp.path().join("modules/app"),
                r#"
                [package]
                name = "team/app"
                version = "1.0.0"
                registry = "https://example.com/index"
                realm = "shared"

                [dependencies]
                Util = "ext/util@1.0.0"
                "#,
            );

            let workspace = Workspace::load(tmp.path())?;
            let registry = InMemoryRegistry::new();
            registry.publish(PackageBuilder::new("ext/util@1.0.0"));
            let sources = PackageSourceMap::new(Box::new(registry.source()));

            let first = resolve_workspace(&workspace, &Default::default(), &sources)?;

            // Publish a newer version, but bias toward old
            registry.publish(PackageBuilder::new("ext/util@1.1.0"));
            let biased = resolve_workspace(&workspace, &first.activated, &sources)?;

            let util_id = find_package_id(&biased, "ext/util");
            assert_eq!(
                util_id.version().to_string(),
                "1.0.0",
                "should prefer the try_to_use version"
            );

            Ok(())
        }

        /// Mutual path deps between members.
        #[test]
        fn workspace_mutual_path_deps() -> anyhow::Result<()> {
            let tmp = TempDir::new()?;
            write_manifest(
                tmp.path(),
                r#"
                [workspace]
                members = ["modules/*"]
                registry = "https://example.com/index"
                realm = "shared"
                "#,
            );
            write_manifest(
                &tmp.path().join("modules/a"),
                r#"
                [package]
                name = "team/a"
                version = "1.0.0"
                registry = "https://example.com/index"
                realm = "shared"

                [dependencies]
                B = { path = "../b" }
                "#,
            );
            write_manifest(
                &tmp.path().join("modules/b"),
                r#"
                [package]
                name = "team/b"
                version = "1.0.0"
                registry = "https://example.com/index"
                realm = "shared"

                [dependencies]
                A = { path = "../a" }
                "#,
            );

            let workspace = Workspace::load(tmp.path())?;
            let registry = InMemoryRegistry::new();
            let sources = PackageSourceMap::new(Box::new(registry.source()));
            let resolved = resolve_workspace(&workspace, &Default::default(), &sources)?;

            assert_eq!(resolved.activated.len(), 2);

            let a_id = find_package_id(&resolved, "team/a");
            let b_id = find_package_id(&resolved, "team/b");

            let a_deps = &resolved.shared_dependencies[&a_id];
            assert_eq!(a_deps.get("B"), Some(&b_id));

            let b_deps = &resolved.shared_dependencies[&b_id];
            assert_eq!(b_deps.get("A"), Some(&a_id));

            Ok(())
        }

        /// Single-package project through resolve_workspace produces same
        /// results as through resolve.
        #[test]
        fn workspace_single_package_matches_resolve() -> anyhow::Result<()> {
            let tmp = TempDir::new()?;
            write_manifest(
                tmp.path(),
                r#"
                [package]
                name = "biff/minimal"
                version = "0.1.0"
                registry = "https://example.com/index"
                realm = "shared"

                [dependencies]
                Dep = "ext/dep@1.0.0"
                "#,
            );

            let workspace = Workspace::load(tmp.path())?;
            let registry = InMemoryRegistry::new();
            registry.publish(PackageBuilder::new("ext/dep@1.0.0"));
            let sources = PackageSourceMap::new(Box::new(registry.source()));

            let ws_resolved = resolve_workspace(&workspace, &Default::default(), &sources)?;

            // The same packages should be activated
            assert_eq!(ws_resolved.activated.len(), 2);
            assert_activated(&ws_resolved, "biff/minimal");
            assert_activated(&ws_resolved, "ext/dep");

            let root_id = find_package_id(&ws_resolved, "biff/minimal");
            let dep_id = find_package_id(&ws_resolved, "ext/dep");

            let root_deps = &ws_resolved.shared_dependencies[&root_id];
            assert_eq!(root_deps.get("Dep"), Some(&dep_id));

            // Single-package root uses DefaultRegistry (identical to legacy resolve())
            assert_eq!(
                ws_resolved.metadata[&root_id].source_registry,
                PackageSourceId::DefaultRegistry
            );
            assert!(!ws_resolved.metadata[&root_id].is_workspace_member);
            // Dep should come from the registry
            assert_eq!(
                ws_resolved.metadata[&dep_id].source_registry,
                PackageSourceId::DefaultRegistry
            );

            Ok(())
        }
    }
}
