use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, bail, Context};
use semver::Version;
use serde::Deserialize;

use crate::dependency_spec::DependencySpec;
use crate::manifest::{
    Manifest, Package, PlaceInfo, ProjectManifest, Realm, WorkspaceInheritable, WorkspaceMetadata,
    MANIFEST_FILE_NAME,
};
use crate::package_name::PackageName;

/// A workspace is a collection of one or more Wally packages that are developed
/// together. A single-package project is transparently represented as a
/// one-member workspace.
#[derive(Debug)]
pub struct Workspace {
    root: PathBuf,
    members: BTreeMap<PathBuf, Manifest>,
    default_member: Option<PackageName>,
    registry: String,
    place: PlaceInfo,
    single_package: bool,
}

impl Workspace {
    /// Load a workspace from a directory containing a `wally.toml`.
    ///
    /// If the manifest contains a `[workspace]` section, members are discovered
    /// via the glob patterns in `members` and their manifests are loaded with
    /// workspace inheritance resolved.
    ///
    /// If no `[workspace]` section is present, the manifest is treated as a
    /// single-package project and wrapped in a one-member workspace.
    pub fn load(dir: &Path) -> anyhow::Result<Self> {
        let root = dir
            .canonicalize()
            .with_context(|| format!("failed to canonicalize workspace root: {}", dir.display()))?;

        let manifest_path = root.join(MANIFEST_FILE_NAME);
        let content = fs_err::read_to_string(&manifest_path)
            .with_context(|| format!("failed to read {}", manifest_path.display()))?;

        let project: ProjectManifest = toml::from_str(&content)
            .with_context(|| format!("failed to parse {}", manifest_path.display()))?;

        match (&project.package, &project.workspace) {
            (_, Some(ws_meta)) => Self::load_workspace(root, &project, ws_meta),
            (Some(_), None) => Self::load_single_package(root, &project),
            (None, None) => bail!("manifest has neither [package] nor [workspace]"),
        }
    }

    fn load_workspace(
        root: PathBuf,
        project: &ProjectManifest,
        ws_meta: &WorkspaceMetadata,
    ) -> anyhow::Result<Self> {
        let mut members = BTreeMap::new();

        let member_dirs = discover_members(&root, &ws_meta.members)?;

        for member_dir in &member_dirs {
            let manifest = load_member_manifest(member_dir, ws_meta)
                .with_context(|| format!("failed to load member at {}", member_dir.display()))?;
            members.insert(member_dir.clone(), manifest);
        }

        // Hybrid root: if the root manifest also has [package], include it as a member.
        if let Some(ref package) = project.package {
            let root_manifest = Manifest {
                package: package.clone(),
                place: resolve_place(&project.place, &ws_meta.place),
                dependencies: project.dependencies.clone(),
                server_dependencies: project.server_dependencies.clone(),
                dev_dependencies: project.dev_dependencies.clone(),
            };
            members.insert(root.clone(), root_manifest);
        }

        if members.is_empty() {
            bail!("workspace has no members: no directories matched the member patterns");
        }

        let registry = ws_meta
            .registry
            .clone()
            .or_else(|| project.package.as_ref().map(|p| p.registry.clone()))
            .ok_or_else(|| {
                anyhow!(
                    "workspace must specify a registry in [workspace] \
                     or have a root [package] with a registry"
                )
            })?;

        let workspace = Workspace {
            root,
            members,
            default_member: ws_meta.default_member.clone(),
            registry,
            place: ws_meta.place.clone(),
            single_package: false,
        };

        workspace.validate()?;
        Ok(workspace)
    }

    fn load_single_package(root: PathBuf, project: &ProjectManifest) -> anyhow::Result<Self> {
        let package = project
            .package
            .clone()
            .ok_or_else(|| anyhow!("manifest has neither [package] nor [workspace]"))?;

        let registry = package.registry.clone();
        let place = project.place.clone();

        let manifest = Manifest {
            package,
            place: project.place.clone(),
            dependencies: project.dependencies.clone(),
            server_dependencies: project.server_dependencies.clone(),
            dev_dependencies: project.dev_dependencies.clone(),
        };

        let mut members = BTreeMap::new();
        members.insert(root.clone(), manifest);

        Ok(Workspace {
            root,
            members,
            default_member: None,
            registry,
            place,
            single_package: true,
        })
    }

    fn validate(&self) -> anyhow::Result<()> {
        let mut seen_names: BTreeMap<String, &Path> = BTreeMap::new();
        for (path, manifest) in &self.members {
            let name = manifest.package.name.to_string();
            if let Some(existing_path) = seen_names.get(&name) {
                bail!(
                    "duplicate package name '{}' found at {} and {}",
                    name,
                    existing_path.display(),
                    path.display()
                );
            }
            seen_names.insert(name, path);
        }

        if let Some(ref default) = self.default_member {
            if self.find_member_by_name(default).is_none() {
                bail!(
                    "default-member '{}' does not match any workspace member",
                    default
                );
            }
        }

        Ok(())
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn members(&self) -> &BTreeMap<PathBuf, Manifest> {
        &self.members
    }

    pub fn default_member(&self) -> Option<&PackageName> {
        self.default_member.as_ref()
    }

    pub fn registry(&self) -> &str {
        &self.registry
    }

    pub fn place(&self) -> &PlaceInfo {
        &self.place
    }

    pub fn find_member_by_name(&self, name: &PackageName) -> Option<(&Path, &Manifest)> {
        self.members
            .iter()
            .find(|(_, m)| &m.package.name == name)
            .map(|(p, m)| (p.as_path(), m))
    }

    pub fn get_member_at_path(&self, path: &Path) -> Option<&Manifest> {
        if let Some(m) = self.members.get(path) {
            return Some(m);
        }
        if let Ok(canonical) = path.canonicalize() {
            return self.members.get(&canonical);
        }
        None
    }

    /// Returns `true` if this workspace was loaded from a single-package
    /// manifest (no `[workspace]` section).
    pub fn is_single_package(&self) -> bool {
        self.single_package
    }

    /// Discover the workspace root starting from `start_dir`.
    ///
    /// Walks up from `start_dir` looking for a `wally.toml` with a
    /// `[workspace]` section. If found, returns that directory. Otherwise,
    /// returns `start_dir` itself (the nearest manifest is treated as a
    /// single-package project).
    pub fn discover_root(start_dir: &Path) -> anyhow::Result<PathBuf> {
        let start = start_dir
            .canonicalize()
            .with_context(|| format!("failed to canonicalize: {}", start_dir.display()))?;

        let mut current = start.as_path();
        loop {
            let manifest_path = current.join(MANIFEST_FILE_NAME);
            if manifest_path.exists() {
                if let Ok(content) = fs_err::read_to_string(&manifest_path) {
                    if let Ok(project) = toml::from_str::<ProjectManifest>(&content) {
                        if project.workspace.is_some() {
                            return Ok(current.to_path_buf());
                        }
                    }
                }
            }
            match current.parent() {
                Some(parent) => current = parent,
                None => break,
            }
        }

        // No workspace root found; use start_dir as a single-package project.
        Ok(start)
    }
}

/// Discover member directories by expanding glob patterns relative to the
/// workspace root.
fn discover_members(root: &Path, patterns: &[String]) -> anyhow::Result<Vec<PathBuf>> {
    let mut member_dirs = Vec::new();

    for pattern in patterns {
        let full_pattern = root.join(pattern);
        let pattern_str = full_pattern.to_string_lossy().to_string();

        let entries = glob::glob(&pattern_str)
            .with_context(|| format!("invalid glob pattern: {}", pattern))?;

        for entry in entries {
            let path = entry
                .with_context(|| format!("error reading glob match for pattern: {}", pattern))?;

            if path.is_dir() && path.join(MANIFEST_FILE_NAME).exists() {
                let canonical = path
                    .canonicalize()
                    .with_context(|| format!("failed to canonicalize: {}", path.display()))?;
                if !member_dirs.contains(&canonical) {
                    member_dirs.push(canonical);
                }
            }
        }
    }

    member_dirs.sort();
    Ok(member_dirs)
}

// ---------------------------------------------------------------------------
// Raw deserialization types for member manifests that support workspace
// inheritance via `{ workspace = true }`.
// ---------------------------------------------------------------------------

/// Intermediate `[package]` representation where `registry` and `realm` may be
/// inherited from the workspace root.
#[derive(Debug, Deserialize)]
struct RawMemberPackage {
    name: PackageName,
    version: Version,
    #[serde(default)]
    registry: Option<WorkspaceInheritable<String>>,
    #[serde(default)]
    realm: Option<WorkspaceInheritable<Realm>>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    license: Option<String>,
    #[serde(default)]
    authors: Vec<String>,
    #[serde(default)]
    include: Vec<String>,
    #[serde(default)]
    exclude: Vec<String>,
    #[serde(default)]
    private: bool,
    #[serde(default)]
    homepage: Option<String>,
    #[serde(default)]
    repository: Option<String>,
}

/// Intermediate manifest representation for workspace members where dependency
/// entries may use `{ workspace = true }` to inherit from
/// `[workspace.dependencies]`.
///
/// Dependency maps use `DependencySpec` directly because
/// `DependencySpec::Workspace` captures `{ workspace = true }` natively.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
struct RawMemberManifest {
    package: RawMemberPackage,
    #[serde(default)]
    place: PlaceInfo,
    #[serde(default)]
    dependencies: BTreeMap<String, DependencySpec>,
    #[serde(default)]
    server_dependencies: BTreeMap<String, DependencySpec>,
    #[serde(default)]
    dev_dependencies: BTreeMap<String, DependencySpec>,
}

/// Load a member's `wally.toml` and resolve workspace inheritance.
fn load_member_manifest(
    member_dir: &Path,
    ws_meta: &WorkspaceMetadata,
) -> anyhow::Result<Manifest> {
    let manifest_path = member_dir.join(MANIFEST_FILE_NAME);
    let content = fs_err::read_to_string(&manifest_path)
        .with_context(|| format!("failed to read {}", manifest_path.display()))?;

    let raw: RawMemberManifest = toml::from_str(&content)
        .with_context(|| format!("failed to parse {}", manifest_path.display()))?;

    resolve_member_manifest(raw, ws_meta)
}

/// Resolve all inheritable fields in a raw member manifest, producing a
/// standard `Manifest` with concrete values.
fn resolve_member_manifest(
    raw: RawMemberManifest,
    ws_meta: &WorkspaceMetadata,
) -> anyhow::Result<Manifest> {
    let registry = resolve_inheritable_or_default(
        raw.package.registry,
        ws_meta.registry.as_ref(),
        "registry",
    )?;

    let realm = resolve_inheritable_or_default(raw.package.realm, ws_meta.realm.as_ref(), "realm")?;

    let place = resolve_place(&raw.place, &ws_meta.place);

    let dependencies = resolve_dep_map(raw.dependencies, &ws_meta.dependencies)
        .context("failed to resolve [dependencies]")?;
    let server_dependencies = resolve_dep_map(raw.server_dependencies, &ws_meta.dependencies)
        .context("failed to resolve [server-dependencies]")?;
    let dev_dependencies = resolve_dep_map(raw.dev_dependencies, &ws_meta.dependencies)
        .context("failed to resolve [dev-dependencies]")?;

    Ok(Manifest {
        package: Package {
            name: raw.package.name,
            version: raw.package.version,
            registry,
            realm,
            description: raw.package.description,
            license: raw.package.license,
            authors: raw.package.authors,
            include: raw.package.include,
            exclude: raw.package.exclude,
            private: raw.package.private,
            homepage: raw.package.homepage,
            repository: raw.package.repository,
        },
        place,
        dependencies,
        server_dependencies,
        dev_dependencies,
    })
}

/// Resolve a field that may be defined locally, explicitly inherited via
/// `{ workspace = true }`, or omitted (implicitly inherited).
fn resolve_inheritable_or_default<T: Clone>(
    local: Option<WorkspaceInheritable<T>>,
    workspace_value: Option<&T>,
    field_name: &str,
) -> anyhow::Result<T> {
    match local {
        Some(WorkspaceInheritable::Defined(v)) => Ok(v),
        Some(WorkspaceInheritable::Workspace { workspace: true }) => workspace_value
            .cloned()
            .ok_or_else(|| {
                anyhow!(
                    "field '{}' is set to `workspace = true` but the workspace root does not define it",
                    field_name
                )
            }),
        Some(WorkspaceInheritable::Workspace { workspace: false }) => Err(anyhow!(
            "field '{}' has `workspace = false` which is not a valid directive",
            field_name
        )),
        None => workspace_value.cloned().ok_or_else(|| {
            anyhow!(
                "field '{}' is not set and the workspace root does not provide a default",
                field_name
            )
        }),
    }
}

/// Merge a member's `PlaceInfo` with workspace defaults: member values take
/// precedence, workspace values fill in gaps.
fn resolve_place(member: &PlaceInfo, workspace: &PlaceInfo) -> PlaceInfo {
    PlaceInfo {
        shared_packages: member
            .shared_packages
            .clone()
            .or_else(|| workspace.shared_packages.clone()),
        server_packages: member
            .server_packages
            .clone()
            .or_else(|| workspace.server_packages.clone()),
    }
}

/// Resolve a dependency map, replacing `DependencySpec::Workspace` entries
/// with the corresponding entry from `[workspace.dependencies]`.
fn resolve_dep_map(
    deps: BTreeMap<String, DependencySpec>,
    ws_deps: &BTreeMap<String, DependencySpec>,
) -> anyhow::Result<BTreeMap<String, DependencySpec>> {
    let mut resolved = BTreeMap::new();

    for (name, spec) in deps {
        let dep = match spec {
            DependencySpec::Workspace { workspace: true } => ws_deps
                .get(&name)
                .cloned()
                .ok_or_else(|| {
                    anyhow!(
                        "dependency '{}' is set to `workspace = true` but is not defined \
                         in [workspace.dependencies]",
                        name
                    )
                })?,
            DependencySpec::Workspace { workspace: false } => {
                bail!(
                    "dependency '{}' has `workspace = false` which is not a valid directive",
                    name
                );
            }
            other => other,
        };
        resolved.insert(name, dep);
    }

    Ok(resolved)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn write_manifest(dir: &Path, content: &str) {
        fs::create_dir_all(dir).unwrap();
        fs::write(dir.join("wally.toml"), content).unwrap();
    }

    // -----------------------------------------------------------------------
    // Single-package wrapping
    // -----------------------------------------------------------------------

    #[test]
    fn single_package_wraps_correctly() {
        let tmp = TempDir::new().unwrap();
        write_manifest(
            tmp.path(),
            r#"
            [package]
            name = "test/my-package"
            version = "1.0.0"
            registry = "https://github.com/UpliftGames/wally-index"
            realm = "shared"
        "#,
        );

        let ws = Workspace::load(tmp.path()).unwrap();
        assert!(ws.is_single_package());
        assert_eq!(ws.members().len(), 1);
        assert_eq!(ws.registry(), "https://github.com/UpliftGames/wally-index");
        assert!(ws.default_member().is_none());

        let (path, manifest) = ws.members().iter().next().unwrap();
        assert_eq!(*path, tmp.path().canonicalize().unwrap());
        assert_eq!(manifest.package.name.to_string(), "test/my-package");
    }

    // -----------------------------------------------------------------------
    // Multi-member workspace with globs
    // -----------------------------------------------------------------------

    #[test]
    fn multi_member_workspace() {
        let tmp = TempDir::new().unwrap();
        write_manifest(
            tmp.path(),
            r#"
            [workspace]
            members = ["packages/*"]
            registry = "https://github.com/UpliftGames/wally-index"
            realm = "shared"
        "#,
        );

        write_manifest(
            &tmp.path().join("packages/foo"),
            r#"
            [package]
            name = "team/foo"
            version = "1.0.0"
            registry = "https://github.com/UpliftGames/wally-index"
            realm = "shared"
        "#,
        );

        write_manifest(
            &tmp.path().join("packages/bar"),
            r#"
            [package]
            name = "team/bar"
            version = "2.0.0"
            registry = "https://github.com/UpliftGames/wally-index"
            realm = "server"
        "#,
        );

        let ws = Workspace::load(tmp.path()).unwrap();
        assert!(!ws.is_single_package());
        assert_eq!(ws.members().len(), 2);
        assert_eq!(ws.registry(), "https://github.com/UpliftGames/wally-index");

        let foo_name: PackageName = "team/foo".parse().unwrap();
        let (_, foo) = ws.find_member_by_name(&foo_name).unwrap();
        assert_eq!(foo.package.version.to_string(), "1.0.0");

        let bar_name: PackageName = "team/bar".parse().unwrap();
        let (_, bar) = ws.find_member_by_name(&bar_name).unwrap();
        assert_eq!(bar.package.version.to_string(), "2.0.0");
        assert_eq!(bar.package.realm, Realm::Server);
    }

    // -----------------------------------------------------------------------
    // Hybrid root ([workspace] + [package])
    // -----------------------------------------------------------------------

    #[test]
    fn hybrid_root() {
        let tmp = TempDir::new().unwrap();
        write_manifest(
            tmp.path(),
            r#"
            [package]
            name = "team/mono-root"
            version = "0.0.0"
            registry = "https://github.com/UpliftGames/wally-index"
            realm = "server"
            private = true

            [workspace]
            members = ["packages/*"]
            registry = "https://github.com/UpliftGames/wally-index"
        "#,
        );

        write_manifest(
            &tmp.path().join("packages/child"),
            r#"
            [package]
            name = "team/child"
            version = "1.0.0"
            registry = "https://github.com/UpliftGames/wally-index"
            realm = "shared"
        "#,
        );

        let ws = Workspace::load(tmp.path()).unwrap();
        assert!(!ws.is_single_package());
        assert_eq!(ws.members().len(), 2);

        let root_name: PackageName = "team/mono-root".parse().unwrap();
        let (root_path, root_manifest) = ws.find_member_by_name(&root_name).unwrap();
        assert_eq!(root_path, tmp.path().canonicalize().unwrap());
        assert!(root_manifest.package.private);

        let child_name: PackageName = "team/child".parse().unwrap();
        assert!(ws.find_member_by_name(&child_name).is_some());
    }

    // -----------------------------------------------------------------------
    // Virtual workspace (no [package])
    // -----------------------------------------------------------------------

    #[test]
    fn virtual_workspace() {
        let tmp = TempDir::new().unwrap();
        write_manifest(
            tmp.path(),
            r#"
            [workspace]
            members = ["packages/*"]
            registry = "https://github.com/UpliftGames/wally-index"
            realm = "shared"
        "#,
        );

        write_manifest(
            &tmp.path().join("packages/alpha"),
            r#"
            [package]
            name = "team/alpha"
            version = "1.0.0"
            registry = "https://github.com/UpliftGames/wally-index"
            realm = "shared"
        "#,
        );

        let ws = Workspace::load(tmp.path()).unwrap();
        assert!(!ws.is_single_package());
        assert_eq!(ws.members().len(), 1);

        let name: PackageName = "team/alpha".parse().unwrap();
        assert!(ws.find_member_by_name(&name).is_some());
    }

    // -----------------------------------------------------------------------
    // { workspace = true } inheritance for deps and metadata
    // -----------------------------------------------------------------------

    #[test]
    fn workspace_inheritance_deps_and_metadata() {
        let tmp = TempDir::new().unwrap();
        write_manifest(
            tmp.path(),
            r#"
            [workspace]
            members = ["packages/*"]
            registry = "https://github.com/UpliftGames/wally-index"
            realm = "shared"

            [workspace.dependencies]
            Roact = "roblox/roact@1.4.0"
            Promise = "evaera/promise@3.0.0"
        "#,
        );

        write_manifest(
            &tmp.path().join("packages/lib-a"),
            r#"
            [package]
            name = "team/lib-a"
            version = "1.0.0"
            realm = { workspace = true }

            [dependencies]
            Roact = { workspace = true }
            Promise = { workspace = true }
        "#,
        );

        let ws = Workspace::load(tmp.path()).unwrap();
        let name: PackageName = "team/lib-a".parse().unwrap();
        let (_, manifest) = ws.find_member_by_name(&name).unwrap();

        assert_eq!(
            manifest.package.registry,
            "https://github.com/UpliftGames/wally-index"
        );
        assert_eq!(manifest.package.realm, Realm::Shared);

        assert_eq!(manifest.dependencies.len(), 2);
        let roact = manifest.dependencies.get("Roact").unwrap();
        assert_eq!(roact.expect_registry().name().scope(), "roblox");
        let promise = manifest.dependencies.get("Promise").unwrap();
        assert_eq!(promise.expect_registry().name().scope(), "evaera");
    }

    #[test]
    fn workspace_inheritance_member_overrides_metadata() {
        let tmp = TempDir::new().unwrap();
        write_manifest(
            tmp.path(),
            r#"
            [workspace]
            members = ["packages/*"]
            registry = "https://default-registry.example.com"
            realm = "shared"
        "#,
        );

        write_manifest(
            &tmp.path().join("packages/custom"),
            r#"
            [package]
            name = "team/custom"
            version = "1.0.0"
            registry = "https://custom-registry.example.com"
            realm = "server"
        "#,
        );

        let ws = Workspace::load(tmp.path()).unwrap();
        let name: PackageName = "team/custom".parse().unwrap();
        let (_, manifest) = ws.find_member_by_name(&name).unwrap();

        assert_eq!(
            manifest.package.registry,
            "https://custom-registry.example.com"
        );
        assert_eq!(manifest.package.realm, Realm::Server);
    }

    #[test]
    fn workspace_inheritance_omitted_fields_inherit() {
        let tmp = TempDir::new().unwrap();
        write_manifest(
            tmp.path(),
            r#"
            [workspace]
            members = ["packages/*"]
            registry = "https://github.com/UpliftGames/wally-index"
            realm = "server"
        "#,
        );

        // Member omits registry and realm entirely -> inherits from workspace
        write_manifest(
            &tmp.path().join("packages/implicit"),
            r#"
            [package]
            name = "team/implicit"
            version = "1.0.0"
        "#,
        );

        let ws = Workspace::load(tmp.path()).unwrap();
        let name: PackageName = "team/implicit".parse().unwrap();
        let (_, manifest) = ws.find_member_by_name(&name).unwrap();

        assert_eq!(
            manifest.package.registry,
            "https://github.com/UpliftGames/wally-index"
        );
        assert_eq!(manifest.package.realm, Realm::Server);
    }

    // -----------------------------------------------------------------------
    // Error: inherit from workspace when workspace doesn't define the field
    // -----------------------------------------------------------------------

    #[test]
    fn error_inherit_missing_registry() {
        let tmp = TempDir::new().unwrap();
        write_manifest(
            tmp.path(),
            r#"
            [workspace]
            members = ["packages/*"]
            realm = "shared"
        "#,
        );

        write_manifest(
            &tmp.path().join("packages/needs-registry"),
            r#"
            [package]
            name = "team/needs-registry"
            version = "1.0.0"
            realm = "shared"
        "#,
        );

        let result = Workspace::load(tmp.path());
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("registry"), "error should mention registry: {}", err);
    }

    #[test]
    fn error_inherit_missing_realm() {
        let tmp = TempDir::new().unwrap();
        write_manifest(
            tmp.path(),
            r#"
            [workspace]
            members = ["packages/*"]
            registry = "https://github.com/UpliftGames/wally-index"
        "#,
        );

        write_manifest(
            &tmp.path().join("packages/needs-realm"),
            r#"
            [package]
            name = "team/needs-realm"
            version = "1.0.0"
        "#,
        );

        let result = Workspace::load(tmp.path());
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("realm"), "error should mention realm: {}", err);
    }

    #[test]
    fn error_inherit_dep_not_in_workspace() {
        let tmp = TempDir::new().unwrap();
        write_manifest(
            tmp.path(),
            r#"
            [workspace]
            members = ["packages/*"]
            registry = "https://github.com/UpliftGames/wally-index"
            realm = "shared"
        "#,
        );

        write_manifest(
            &tmp.path().join("packages/bad"),
            r#"
            [package]
            name = "team/bad"
            version = "1.0.0"

            [dependencies]
            Missing = { workspace = true }
        "#,
        );

        let result = Workspace::load(tmp.path());
        assert!(result.is_err());
        let err = format!("{:#}", result.unwrap_err());
        assert!(
            err.contains("Missing"),
            "error should mention the dep name: {}",
            err
        );
    }

    // -----------------------------------------------------------------------
    // Error: no members found, duplicate names, invalid default_member
    // -----------------------------------------------------------------------

    #[test]
    fn error_no_members_virtual_workspace() {
        let tmp = TempDir::new().unwrap();
        write_manifest(
            tmp.path(),
            r#"
            [workspace]
            members = ["nonexistent/*"]
            registry = "https://github.com/UpliftGames/wally-index"
        "#,
        );

        let result = Workspace::load(tmp.path());
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("no members"));
    }

    #[test]
    fn error_duplicate_names() {
        let tmp = TempDir::new().unwrap();
        write_manifest(
            tmp.path(),
            r#"
            [workspace]
            members = ["packages/*"]
            registry = "https://github.com/UpliftGames/wally-index"
            realm = "shared"
        "#,
        );

        write_manifest(
            &tmp.path().join("packages/first"),
            r#"
            [package]
            name = "team/dupe"
            version = "1.0.0"
        "#,
        );

        write_manifest(
            &tmp.path().join("packages/second"),
            r#"
            [package]
            name = "team/dupe"
            version = "2.0.0"
        "#,
        );

        let result = Workspace::load(tmp.path());
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("duplicate"),
            "error should mention duplicate: {}",
            err
        );
    }

    #[test]
    fn error_invalid_default_member() {
        let tmp = TempDir::new().unwrap();
        write_manifest(
            tmp.path(),
            r#"
            [workspace]
            members = ["packages/*"]
            default-member = "team/nonexistent"
            registry = "https://github.com/UpliftGames/wally-index"
            realm = "shared"
        "#,
        );

        write_manifest(
            &tmp.path().join("packages/actual"),
            r#"
            [package]
            name = "team/actual"
            version = "1.0.0"
        "#,
        );

        let result = Workspace::load(tmp.path());
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("default-member"),
            "error should mention default-member: {}",
            err
        );
    }

    // -----------------------------------------------------------------------
    // find_member_by_name, get_member_at_path, is_single_package
    // -----------------------------------------------------------------------

    #[test]
    fn find_member_by_name() {
        let tmp = TempDir::new().unwrap();
        write_manifest(
            tmp.path(),
            r#"
            [workspace]
            members = ["packages/*"]
            registry = "https://github.com/UpliftGames/wally-index"
            realm = "shared"
        "#,
        );

        write_manifest(
            &tmp.path().join("packages/needle"),
            r#"
            [package]
            name = "team/needle"
            version = "1.0.0"
        "#,
        );

        write_manifest(
            &tmp.path().join("packages/other"),
            r#"
            [package]
            name = "team/other"
            version = "2.0.0"
        "#,
        );

        let ws = Workspace::load(tmp.path()).unwrap();

        let name: PackageName = "team/needle".parse().unwrap();
        let result = ws.find_member_by_name(&name);
        assert!(result.is_some());
        let (_, manifest) = result.unwrap();
        assert_eq!(manifest.package.version.to_string(), "1.0.0");

        let missing: PackageName = "team/missing".parse().unwrap();
        assert!(ws.find_member_by_name(&missing).is_none());
    }

    #[test]
    fn get_member_at_path() {
        let tmp = TempDir::new().unwrap();
        write_manifest(
            tmp.path(),
            r#"
            [workspace]
            members = ["packages/*"]
            registry = "https://github.com/UpliftGames/wally-index"
            realm = "shared"
        "#,
        );

        write_manifest(
            &tmp.path().join("packages/target"),
            r#"
            [package]
            name = "team/target"
            version = "3.0.0"
        "#,
        );

        let ws = Workspace::load(tmp.path()).unwrap();

        // Lookup by the non-canonical (relative) path should still work
        let member_path = tmp.path().join("packages/target");
        let manifest = ws.get_member_at_path(&member_path);
        assert!(manifest.is_some());
        assert_eq!(manifest.unwrap().package.name.to_string(), "team/target");

        // Non-existent path
        let bad = tmp.path().join("packages/nope");
        assert!(ws.get_member_at_path(&bad).is_none());
    }

    #[test]
    fn is_single_package_true_for_plain_manifest() {
        let tmp = TempDir::new().unwrap();
        write_manifest(
            tmp.path(),
            r#"
            [package]
            name = "test/single"
            version = "1.0.0"
            registry = "https://github.com/UpliftGames/wally-index"
            realm = "shared"
        "#,
        );

        let ws = Workspace::load(tmp.path()).unwrap();
        assert!(ws.is_single_package());
    }

    #[test]
    fn is_single_package_false_for_workspace() {
        let tmp = TempDir::new().unwrap();
        write_manifest(
            tmp.path(),
            r#"
            [workspace]
            members = ["packages/*"]
            registry = "https://github.com/UpliftGames/wally-index"
            realm = "shared"
        "#,
        );

        write_manifest(
            &tmp.path().join("packages/only"),
            r#"
            [package]
            name = "team/only"
            version = "1.0.0"
        "#,
        );

        let ws = Workspace::load(tmp.path()).unwrap();
        assert!(!ws.is_single_package());
    }

    // -----------------------------------------------------------------------
    // Registry and place inheritance
    // -----------------------------------------------------------------------

    #[test]
    fn place_inheritance() {
        let tmp = TempDir::new().unwrap();
        write_manifest(
            tmp.path(),
            r#"
            [workspace]
            members = ["packages/*"]
            registry = "https://github.com/UpliftGames/wally-index"
            realm = "shared"

            [workspace.place]
            shared-packages = "game.ReplicatedStorage.Packages"
            server-packages = "game.ServerScriptService.Packages"
        "#,
        );

        // Member with no [place] -> inherits workspace place
        write_manifest(
            &tmp.path().join("packages/inherits"),
            r#"
            [package]
            name = "team/inherits"
            version = "1.0.0"
        "#,
        );

        // Member that overrides shared-packages only
        write_manifest(
            &tmp.path().join("packages/overrides"),
            r#"
            [package]
            name = "team/overrides"
            version = "1.0.0"

            [place]
            shared-packages = "game.Workspace.CustomPackages"
        "#,
        );

        let ws = Workspace::load(tmp.path()).unwrap();

        let name_inherits: PackageName = "team/inherits".parse().unwrap();
        let (_, m_inherits) = ws.find_member_by_name(&name_inherits).unwrap();
        assert_eq!(
            m_inherits.place.shared_packages.as_deref(),
            Some("game.ReplicatedStorage.Packages")
        );
        assert_eq!(
            m_inherits.place.server_packages.as_deref(),
            Some("game.ServerScriptService.Packages")
        );

        let name_overrides: PackageName = "team/overrides".parse().unwrap();
        let (_, m_overrides) = ws.find_member_by_name(&name_overrides).unwrap();
        assert_eq!(
            m_overrides.place.shared_packages.as_deref(),
            Some("game.Workspace.CustomPackages")
        );
        assert_eq!(
            m_overrides.place.server_packages.as_deref(),
            Some("game.ServerScriptService.Packages")
        );
    }

    #[test]
    fn workspace_place_propagates_to_workspace_struct() {
        let tmp = TempDir::new().unwrap();
        write_manifest(
            tmp.path(),
            r#"
            [workspace]
            members = ["packages/*"]
            registry = "https://github.com/UpliftGames/wally-index"
            realm = "shared"

            [workspace.place]
            shared-packages = "game.ReplicatedStorage.Packages"
        "#,
        );

        write_manifest(
            &tmp.path().join("packages/a"),
            r#"
            [package]
            name = "team/a"
            version = "1.0.0"
        "#,
        );

        let ws = Workspace::load(tmp.path()).unwrap();
        assert_eq!(
            ws.place().shared_packages.as_deref(),
            Some("game.ReplicatedStorage.Packages")
        );
    }

    // -----------------------------------------------------------------------
    // Default member
    // -----------------------------------------------------------------------

    #[test]
    fn valid_default_member() {
        let tmp = TempDir::new().unwrap();
        write_manifest(
            tmp.path(),
            r#"
            [workspace]
            members = ["packages/*"]
            default-member = "team/primary"
            registry = "https://github.com/UpliftGames/wally-index"
            realm = "shared"
        "#,
        );

        write_manifest(
            &tmp.path().join("packages/primary"),
            r#"
            [package]
            name = "team/primary"
            version = "1.0.0"
        "#,
        );

        write_manifest(
            &tmp.path().join("packages/secondary"),
            r#"
            [package]
            name = "team/secondary"
            version = "1.0.0"
        "#,
        );

        let ws = Workspace::load(tmp.path()).unwrap();
        assert_eq!(ws.default_member().unwrap().to_string(), "team/primary");
    }

    // -----------------------------------------------------------------------
    // Error: no [package] and no [workspace]
    // -----------------------------------------------------------------------

    #[test]
    fn error_neither_package_nor_workspace() {
        let tmp = TempDir::new().unwrap();
        write_manifest(
            tmp.path(),
            r#"
            [place]
            shared-packages = "game.ReplicatedStorage.Packages"
        "#,
        );

        let result = Workspace::load(tmp.path());
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("neither"));
    }

    // -----------------------------------------------------------------------
    // Hybrid root inherits registry from [workspace] for workspace struct
    // -----------------------------------------------------------------------

    #[test]
    fn hybrid_root_registry_from_workspace() {
        let tmp = TempDir::new().unwrap();
        write_manifest(
            tmp.path(),
            r#"
            [package]
            name = "team/root"
            version = "0.0.0"
            registry = "https://pkg-registry.example.com"
            realm = "server"

            [workspace]
            members = ["packages/*"]
            registry = "https://ws-registry.example.com"
            realm = "shared"
        "#,
        );

        write_manifest(
            &tmp.path().join("packages/child"),
            r#"
            [package]
            name = "team/child"
            version = "1.0.0"
        "#,
        );

        let ws = Workspace::load(tmp.path()).unwrap();
        assert_eq!(ws.registry(), "https://ws-registry.example.com");
    }

    #[test]
    fn hybrid_root_falls_back_to_package_registry() {
        let tmp = TempDir::new().unwrap();
        write_manifest(
            tmp.path(),
            r#"
            [package]
            name = "team/root"
            version = "0.0.0"
            registry = "https://pkg-registry.example.com"
            realm = "server"

            [workspace]
            members = ["packages/*"]
            realm = "shared"
        "#,
        );

        // Child specifies its own registry since the workspace doesn't provide one.
        // This test verifies the *Workspace struct's* registry field falls back
        // to [package].registry when [workspace].registry is absent.
        write_manifest(
            &tmp.path().join("packages/child"),
            r#"
            [package]
            name = "team/child"
            version = "1.0.0"
            registry = "https://child-registry.example.com"
            realm = "shared"
        "#,
        );

        let ws = Workspace::load(tmp.path()).unwrap();
        assert_eq!(ws.registry(), "https://pkg-registry.example.com");
    }

    // -----------------------------------------------------------------------
    // Directories without wally.toml are skipped by glob discovery
    // -----------------------------------------------------------------------

    #[test]
    fn glob_skips_dirs_without_manifest() {
        let tmp = TempDir::new().unwrap();
        write_manifest(
            tmp.path(),
            r#"
            [workspace]
            members = ["packages/*"]
            registry = "https://github.com/UpliftGames/wally-index"
            realm = "shared"
        "#,
        );

        write_manifest(
            &tmp.path().join("packages/real"),
            r#"
            [package]
            name = "team/real"
            version = "1.0.0"
        "#,
        );

        // Directory without wally.toml
        fs::create_dir_all(tmp.path().join("packages/no-manifest")).unwrap();

        let ws = Workspace::load(tmp.path()).unwrap();
        assert_eq!(ws.members().len(), 1);
    }

    // -----------------------------------------------------------------------
    // Server-dependencies and dev-dependencies inheritance
    // -----------------------------------------------------------------------

    #[test]
    fn workspace_inheritance_server_and_dev_deps() {
        let tmp = TempDir::new().unwrap();
        write_manifest(
            tmp.path(),
            r#"
            [workspace]
            members = ["packages/*"]
            registry = "https://github.com/UpliftGames/wally-index"
            realm = "server"

            [workspace.dependencies]
            Roact = "roblox/roact@1.4.0"
            TestEZ = "roblox/testez@0.4.0"
        "#,
        );

        write_manifest(
            &tmp.path().join("packages/svc"),
            r#"
            [package]
            name = "team/svc"
            version = "1.0.0"

            [server-dependencies]
            Roact = { workspace = true }

            [dev-dependencies]
            TestEZ = { workspace = true }
        "#,
        );

        let ws = Workspace::load(tmp.path()).unwrap();
        let name: PackageName = "team/svc".parse().unwrap();
        let (_, manifest) = ws.find_member_by_name(&name).unwrap();

        assert_eq!(manifest.server_dependencies.len(), 1);
        assert_eq!(
            manifest
                .server_dependencies
                .get("Roact")
                .unwrap()
                .expect_registry()
                .name()
                .scope(),
            "roblox"
        );

        assert_eq!(manifest.dev_dependencies.len(), 1);
        assert_eq!(
            manifest
                .dev_dependencies
                .get("TestEZ")
                .unwrap()
                .expect_registry()
                .name()
                .scope(),
            "roblox"
        );
    }
}
