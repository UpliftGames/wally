use std::{
    collections::BTreeSet,
    fmt::Display,
    io,
    path::{Path, PathBuf},
    time::Duration,
};

use anyhow::{bail, format_err};
use crossterm::style::{Color, SetForegroundColor};
use fs_err as fs;
use indicatif::{ProgressBar, ProgressStyle};
use indoc::{formatdoc, indoc};

use crate::{
    manifest::Realm,
    package_contents::PackageContents,
    package_id::PackageId,
    package_source::{PackageSourceId, PackageSourceMap, PackageSourceProvider},
    resolution::Resolve,
};

/// Distinguishes between the two container directories within each realm's
/// package folder (`Packages/`, `ServerPackages/`, `DevPackages/`).
///
/// - `Index` (`_Index/`) holds unpacked registry packages.
/// - `Workspace` (`_Workspace/`) holds Rojo project references to local
///   workspace members.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackageContainer {
    Index,
    Workspace,
}

impl PackageContainer {
    pub fn dir_name(&self) -> &'static str {
        match self {
            PackageContainer::Index => "_Index",
            PackageContainer::Workspace => "_Workspace",
        }
    }
}

/// Determine which container a resolved package belongs in based on whether
/// it is a workspace member.
fn package_container(resolved: &Resolve, package_id: &PackageId) -> PackageContainer {
    resolved
        .metadata
        .get(package_id)
        .map(|m| {
            if m.is_workspace_member {
                PackageContainer::Workspace
            } else {
                PackageContainer::Index
            }
        })
        .unwrap_or(PackageContainer::Index)
}

#[derive(Clone)]
pub struct InstallationContext {
    shared_dir: PathBuf,
    shared_index_dir: PathBuf,
    shared_workspace_dir: PathBuf,
    shared_path: Option<String>,
    server_dir: PathBuf,
    server_index_dir: PathBuf,
    server_workspace_dir: PathBuf,
    server_path: Option<String>,
    dev_dir: PathBuf,
    dev_index_dir: PathBuf,
    dev_workspace_dir: PathBuf,
}

impl InstallationContext {
    /// Create a new `InstallationContext` for the given path.
    pub fn new(
        project_path: &Path,
        shared_path: Option<String>,
        server_path: Option<String>,
    ) -> Self {
        let shared_dir = project_path.join("Packages");
        let server_dir = project_path.join("ServerPackages");
        let dev_dir = project_path.join("DevPackages");

        let shared_index_dir = shared_dir.join("_Index");
        let server_index_dir = server_dir.join("_Index");
        let dev_index_dir = dev_dir.join("_Index");

        let shared_workspace_dir = shared_dir.join("_Workspace");
        let server_workspace_dir = server_dir.join("_Workspace");
        let dev_workspace_dir = dev_dir.join("_Workspace");

        Self {
            shared_dir,
            shared_index_dir,
            shared_workspace_dir,
            shared_path,
            server_dir,
            server_index_dir,
            server_workspace_dir,
            server_path,
            dev_dir,
            dev_index_dir,
            dev_workspace_dir,
        }
    }

    /// Delete the existing package directories, if they exist.
    pub fn clean(&self) -> anyhow::Result<()> {
        fn remove_ignore_not_found(path: &Path) -> io::Result<()> {
            if let Err(err) = fs::remove_dir_all(path) {
                if err.kind() != io::ErrorKind::NotFound {
                    return Err(err);
                }
            }

            Ok(())
        }

        remove_ignore_not_found(&self.shared_dir)?;
        remove_ignore_not_found(&self.server_dir)?;
        remove_ignore_not_found(&self.dev_dir)?;

        Ok(())
    }

    /// Return the container directory for a given realm and container type.
    fn container_dir(&self, realm: Realm, container: PackageContainer) -> &Path {
        match (realm, container) {
            (Realm::Shared, PackageContainer::Index) => &self.shared_index_dir,
            (Realm::Shared, PackageContainer::Workspace) => &self.shared_workspace_dir,
            (Realm::Server, PackageContainer::Index) => &self.server_index_dir,
            (Realm::Server, PackageContainer::Workspace) => &self.server_workspace_dir,
            (Realm::Dev, PackageContainer::Index) => &self.dev_index_dir,
            (Realm::Dev, PackageContainer::Workspace) => &self.dev_workspace_dir,
        }
    }

    /// Install all packages from the given `Resolve`.
    ///
    /// `root_package_ids` contains the set of packages that are "roots" --
    /// workspace members or the single root in a non-workspace project. Root
    /// packages get top-level links (e.g. `Packages/Foo.lua`). In a
    /// multi-member workspace, roots also get `_Workspace/` entries with a
    /// Rojo `default.project.json` pointing back to their source directory.
    pub fn install(
        self,
        sources: PackageSourceMap,
        root_package_ids: BTreeSet<PackageId>,
        resolved: Resolve,
    ) -> anyhow::Result<()> {
        let mut handles = Vec::new();
        let resolved_copy = resolved.clone();

        let download_count = resolved_copy
            .activated
            .iter()
            .filter(|id| {
                let meta = resolved_copy.metadata.get(id);
                let is_ws = meta.map(|m| m.is_workspace_member).unwrap_or(false);
                let is_root = root_package_ids.contains(id);
                let is_sole_root = is_root && root_package_ids.len() == 1;
                !is_sole_root && !is_ws
            })
            .count() as u64;

        let bar = ProgressBar::new(download_count).with_style(
            ProgressStyle::with_template(
                "{spinner:.cyan.bold} {pos}/{len} [{wide_bar:.cyan/blue}]",
            )
            .unwrap()
            .tick_chars("⠁⠈⠐⠠⠄⠂ ")
            .progress_chars("#>-"),
        );
        bar.enable_steady_tick(Duration::from_millis(100));

        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(50)
            .enable_all()
            .build()
            .unwrap();

        for package_id in resolved_copy.activated {
            log::debug!("Installing {}...", package_id);

            let shared_deps = resolved.shared_dependencies.get(&package_id);
            let server_deps = resolved.server_dependencies.get(&package_id);
            let dev_deps = resolved.dev_dependencies.get(&package_id);

            let is_root = root_package_ids.contains(&package_id);
            let metadata = resolved.metadata.get(&package_id).unwrap();
            let is_workspace_member = metadata.is_workspace_member;
            let is_sole_root = is_root && root_package_ids.len() == 1;

            // Root packages get top-level links from the realm directory
            // (e.g. Packages/Foo.lua -> _Index or _Workspace).
            if is_root {
                if let Some(deps) = shared_deps {
                    self.write_root_package_links(Realm::Shared, deps, &resolved)?;
                }
                if let Some(deps) = server_deps {
                    self.write_root_package_links(Realm::Server, deps, &resolved)?;
                }
                if let Some(deps) = dev_deps {
                    self.write_root_package_links(Realm::Dev, deps, &resolved)?;
                }
            }

            // In a single-package project the sole root is the user's own
            // project -- it doesn't need a container entry or sibling links.
            if is_sole_root {
                continue;
            }

            let package_realm = metadata.origin_realm;

            // Sibling links (inside the package's _Index or _Workspace entry).
            if let Some(deps) = shared_deps {
                self.write_package_links(&package_id, package_realm, deps, &resolved)?;
            }
            if let Some(deps) = server_deps {
                self.write_package_links(&package_id, package_realm, deps, &resolved)?;
            }
            if let Some(deps) = dev_deps {
                self.write_package_links(&package_id, package_realm, deps, &resolved)?;
            }

            if is_workspace_member {
                let member_dir = match &metadata.source_registry {
                    PackageSourceId::Path(p) => p.clone(),
                    _ => unreachable!("workspace member must have Path source"),
                };
                self.write_workspace_project_json(&package_id, package_realm, &member_dir)?;
                bar.println(format!(
                    "{}   Linked {}{}",
                    SetForegroundColor(Color::DarkGreen),
                    SetForegroundColor(Color::Reset),
                    package_id,
                ));
            } else {
                let source_registry =
                    resolved_copy.metadata[&package_id].source_registry.clone();
                let source_copy = sources.clone();
                let context = self.clone();
                let b = bar.clone();
                let pid = package_id.clone();

                let handle = runtime.spawn_blocking(move || {
                    let package_source = source_copy.get(&source_registry).unwrap();
                    let contents = package_source.download_package(&pid)?;
                    b.println(format!(
                        "{} Downloaded {}{}",
                        SetForegroundColor(Color::DarkGreen),
                        SetForegroundColor(Color::Reset),
                        pid,
                    ));
                    b.inc(1);
                    context.write_contents(&pid, &contents, package_realm, PackageContainer::Index)
                });

                handles.push(handle);
            }
        }

        let num_packages = handles.len();

        for handle in handles {
            runtime
                .block_on(handle)
                .expect("Package failed to be installed.")?;
        }

        bar.finish_and_clear();
        log::info!("Downloaded {} packages!", num_packages);

        Ok(())
    }

    /// Link from a realm root directory (e.g. `Packages/Foo.lua`) into the
    /// specified container (`_Index` or `_Workspace`).
    fn link_root(&self, id: &PackageId, container: PackageContainer) -> String {
        formatdoc! {r#"
            return require(script.Parent.{container}["{full_name}"]["{short_name}"])
            "#,
            container = container.dir_name(),
            full_name = package_id_file_name(id),
            short_name = id.name().name()
        }
    }

    /// Link between packages. When `source_container` and `target_container`
    /// are the same, the link navigates within the same container
    /// (`script.Parent.Parent`). When they differ, the link goes up to the
    /// realm root and back down into the target container.
    fn link_sibling(
        &self,
        id: &PackageId,
        source_container: PackageContainer,
        target_container: PackageContainer,
    ) -> String {
        if source_container == target_container {
            formatdoc! {r#"
                return require(script.Parent.Parent["{full_name}"]["{short_name}"])
                "#,
                full_name = package_id_file_name(id),
                short_name = id.name().name()
            }
        } else {
            formatdoc! {r#"
                return require(script.Parent.Parent.Parent.{container}["{full_name}"]["{short_name}"])
                "#,
                container = target_container.dir_name(),
                full_name = package_id_file_name(id),
                short_name = id.name().name()
            }
        }
    }

    /// Cross-realm link into the shared packages directory, targeting the
    /// specified container.
    fn link_shared(&self, id: &PackageId, container: PackageContainer) -> anyhow::Result<String> {
        let shared_path = self.shared_path.as_ref().ok_or_else(|| {
            format_err!(indoc! {r#"
                A server or dev dependency is depending on a shared dependency.
                To link these packages correctly you must declare where shared
                packages are placed in the roblox datamodel in your wally.toml.
                
                This typically looks like:

                [place]
                shared-packages = "game.ReplicatedStorage.Packages"
            "#})
        })?;

        let contents = formatdoc! {r#"
            return require({packages}.{container}["{full_name}"]["{short_name}"])
            "#,
            packages = shared_path,
            container = container.dir_name(),
            full_name = package_id_file_name(id),
            short_name = id.name().name()
        };

        Ok(contents)
    }

    /// Cross-realm link into the server packages directory, targeting the
    /// specified container.
    fn link_server(&self, id: &PackageId, container: PackageContainer) -> anyhow::Result<String> {
        let server_path = self.server_path.as_ref().ok_or_else(|| {
            format_err!(indoc! {r#"
                A dev dependency is depending on a server dependency.
                To link these packages correctly you must declare where server
                packages are placed in the roblox datamodel in your wally.toml.
                
                This typically looks like:

                [place]
                server-packages = "game.ServerScriptService.Packages"
            "#})
        })?;

        let contents = formatdoc! {r#"
            return require({packages}.{container}["{full_name}"]["{short_name}"])
            "#,
            packages = server_path,
            container = container.dir_name(),
            full_name = package_id_file_name(id),
            short_name = id.name().name()
        };

        Ok(contents)
    }

    fn write_root_package_links<'a, K: Display>(
        &self,
        root_realm: Realm,
        dependencies: impl IntoIterator<Item = (K, &'a PackageId)>,
        resolved: &Resolve,
    ) -> anyhow::Result<()> {
        log::debug!("Writing root package links");

        let base_path = match root_realm {
            Realm::Shared => &self.shared_dir,
            Realm::Server => &self.server_dir,
            Realm::Dev => &self.dev_dir,
        };

        log::trace!("Creating directory {}", base_path.display());
        fs::create_dir_all(base_path)?;

        for (dep_name, dep_package_id) in dependencies {
            let dep_meta = resolved.metadata.get(dep_package_id).unwrap();
            let dep_realm = dep_meta.origin_realm;
            let dep_container = package_container(resolved, dep_package_id);
            let path = base_path.join(format!("{}.lua", dep_name));

            let contents = match (root_realm, dep_realm) {
                (source, dest) if source == dest => {
                    self.link_root(dep_package_id, dep_container)
                }
                (_, Realm::Server) => self.link_server(dep_package_id, dep_container)?,
                (_, Realm::Shared) => self.link_shared(dep_package_id, dep_container)?,
                (_, Realm::Dev) => {
                    bail!("A dev dependency cannot be depended upon by a non-dev dependency")
                }
            };

            log::trace!("Writing {}", path.display());
            fs::write(path, contents)?;
        }

        Ok(())
    }

    fn write_package_links<'a, K: std::fmt::Display>(
        &self,
        package_id: &PackageId,
        package_realm: Realm,
        dependencies: impl IntoIterator<Item = (K, &'a PackageId)>,
        resolved: &Resolve,
    ) -> anyhow::Result<()> {
        log::debug!("Writing package links for {}", package_id);

        let source_container = package_container(resolved, package_id);
        let base_path = self
            .container_dir(package_realm, source_container)
            .join(package_id_file_name(package_id));

        log::trace!("Creating directory {}", base_path.display());
        fs::create_dir_all(&base_path)?;

        for (dep_name, dep_package_id) in dependencies {
            let dep_meta = resolved.metadata.get(dep_package_id).unwrap();
            let dep_realm = dep_meta.origin_realm;
            let dep_container = package_container(resolved, dep_package_id);
            let path = base_path.join(format!("{}.lua", dep_name));

            let contents = match (package_realm, dep_realm) {
                (source, dest) if source == dest => {
                    self.link_sibling(dep_package_id, source_container, dep_container)
                }
                (_, Realm::Server) => self.link_server(dep_package_id, dep_container)?,
                (_, Realm::Shared) => self.link_shared(dep_package_id, dep_container)?,
                (_, Realm::Dev) => {
                    bail!("A dev dependency cannot be depended upon by a non-dev dependency")
                }
            };

            log::trace!("Writing {}", path.display());
            fs::write(path, contents)?;
        }

        Ok(())
    }

    fn write_contents(
        &self,
        package_id: &PackageId,
        contents: &PackageContents,
        realm: Realm,
        container: PackageContainer,
    ) -> anyhow::Result<()> {
        let mut path = self.container_dir(realm, container).to_path_buf();

        path.push(package_id_file_name(package_id));
        path.push(package_id.name().name());

        fs::create_dir_all(&path)?;
        contents.unpack_into_path(&path)?;

        Ok(())
    }

    /// Create a Rojo `default.project.json` inside `_Workspace/` that points
    /// back to the workspace member's source directory. This acts as a
    /// platform-safe symlink that Rojo can follow.
    fn write_workspace_project_json(
        &self,
        package_id: &PackageId,
        realm: Realm,
        member_dir: &Path,
    ) -> anyhow::Result<()> {
        let container_dir = self.container_dir(realm, PackageContainer::Workspace);
        let package_dir = container_dir.join(package_id_file_name(package_id));
        let short_name = package_id.name().name();
        let source_dir = package_dir.join(short_name);

        fs::create_dir_all(&source_dir)?;

        // `member_dir` is already absolute (canonicalized during workspace
        // loading). Canonicalize the freshly-created `source_dir` so both
        // sides are absolute before computing the relative path.
        let source_dir_abs = source_dir.canonicalize()?;

        let relative = pathdiff::diff_paths(member_dir, &source_dir_abs)
            .unwrap_or_else(|| member_dir.to_path_buf());

        let path_str = relative.to_string_lossy().replace('\\', "/");

        let project_json = serde_json::json!({
            "name": short_name,
            "tree": {
                "$path": path_str
            }
        });

        fs::write(
            source_dir.join("default.project.json"),
            serde_json::to_string_pretty(&project_json)?,
        )?;

        log::debug!(
            "Wrote workspace project json for {} at {}",
            package_id,
            source_dir.display()
        );

        Ok(())
    }
}

/// Creates a suitable name for use in file paths that refer to this package.
fn package_id_file_name(id: &PackageId) -> String {
    format!(
        "{}_{}@{}",
        id.name().scope(),
        id.name().name(),
        id.version()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn package_container_dir_names() {
        assert_eq!(PackageContainer::Index.dir_name(), "_Index");
        assert_eq!(PackageContainer::Workspace.dir_name(), "_Workspace");
    }

    #[test]
    fn link_root_index() {
        let ctx = InstallationContext::new(Path::new("/project"), None, None);
        let id: PackageId = "biff/cool@1.0.0".parse().unwrap();
        let link = ctx.link_root(&id, PackageContainer::Index);
        assert!(link.contains("_Index"));
        assert!(link.contains("biff_cool@1.0.0"));
        assert!(link.contains(r#"["cool"]"#));
    }

    #[test]
    fn link_root_workspace() {
        let ctx = InstallationContext::new(Path::new("/project"), None, None);
        let id: PackageId = "team/foo@1.0.0".parse().unwrap();
        let link = ctx.link_root(&id, PackageContainer::Workspace);
        assert!(link.contains("_Workspace"));
        assert!(link.contains("team_foo@1.0.0"));
    }

    #[test]
    fn link_sibling_same_container() {
        let ctx = InstallationContext::new(Path::new("/project"), None, None);
        let id: PackageId = "biff/dep@2.0.0".parse().unwrap();
        let link = ctx.link_sibling(&id, PackageContainer::Index, PackageContainer::Index);
        assert!(link.contains("script.Parent.Parent"));
        assert!(!link.contains("_Index"));
        assert!(!link.contains("_Workspace"));
    }

    #[test]
    fn link_sibling_cross_container_workspace_to_index() {
        let ctx = InstallationContext::new(Path::new("/project"), None, None);
        let id: PackageId = "ext/util@1.0.0".parse().unwrap();
        let link = ctx.link_sibling(&id, PackageContainer::Workspace, PackageContainer::Index);
        assert!(link.contains("script.Parent.Parent.Parent._Index"));
        assert!(link.contains("ext_util@1.0.0"));
    }

    #[test]
    fn link_sibling_cross_container_index_to_workspace() {
        let ctx = InstallationContext::new(Path::new("/project"), None, None);
        let id: PackageId = "team/foo@1.0.0".parse().unwrap();
        let link = ctx.link_sibling(&id, PackageContainer::Index, PackageContainer::Workspace);
        assert!(link.contains("script.Parent.Parent.Parent._Workspace"));
        assert!(link.contains("team_foo@1.0.0"));
    }

    #[test]
    fn link_shared_with_container() {
        let ctx = InstallationContext::new(
            Path::new("/project"),
            Some("game.ReplicatedStorage.Packages".to_string()),
            None,
        );
        let id: PackageId = "biff/shared@1.0.0".parse().unwrap();

        let link_index = ctx.link_shared(&id, PackageContainer::Index).unwrap();
        assert!(link_index.contains("game.ReplicatedStorage.Packages._Index"));

        let link_ws = ctx.link_shared(&id, PackageContainer::Workspace).unwrap();
        assert!(link_ws.contains("game.ReplicatedStorage.Packages._Workspace"));
    }

    #[test]
    fn link_server_with_container() {
        let ctx = InstallationContext::new(
            Path::new("/project"),
            None,
            Some("game.ServerScriptService.Packages".to_string()),
        );
        let id: PackageId = "biff/server@1.0.0".parse().unwrap();

        let link_index = ctx.link_server(&id, PackageContainer::Index).unwrap();
        assert!(link_index.contains("game.ServerScriptService.Packages._Index"));

        let link_ws = ctx.link_server(&id, PackageContainer::Workspace).unwrap();
        assert!(link_ws.contains("game.ServerScriptService.Packages._Workspace"));
    }

    #[test]
    fn link_shared_missing_path_errors() {
        let ctx = InstallationContext::new(Path::new("/project"), None, None);
        let id: PackageId = "biff/shared@1.0.0".parse().unwrap();
        assert!(ctx.link_shared(&id, PackageContainer::Index).is_err());
    }

    #[test]
    fn link_server_missing_path_errors() {
        let ctx = InstallationContext::new(Path::new("/project"), None, None);
        let id: PackageId = "biff/server@1.0.0".parse().unwrap();
        assert!(ctx.link_server(&id, PackageContainer::Index).is_err());
    }

    #[test]
    fn container_dir_returns_correct_paths() {
        let ctx = InstallationContext::new(Path::new("/project"), None, None);

        assert_eq!(
            ctx.container_dir(Realm::Shared, PackageContainer::Index),
            Path::new("/project/Packages/_Index")
        );
        assert_eq!(
            ctx.container_dir(Realm::Shared, PackageContainer::Workspace),
            Path::new("/project/Packages/_Workspace")
        );
        assert_eq!(
            ctx.container_dir(Realm::Server, PackageContainer::Index),
            Path::new("/project/ServerPackages/_Index")
        );
        assert_eq!(
            ctx.container_dir(Realm::Server, PackageContainer::Workspace),
            Path::new("/project/ServerPackages/_Workspace")
        );
        assert_eq!(
            ctx.container_dir(Realm::Dev, PackageContainer::Index),
            Path::new("/project/DevPackages/_Index")
        );
        assert_eq!(
            ctx.container_dir(Realm::Dev, PackageContainer::Workspace),
            Path::new("/project/DevPackages/_Workspace")
        );
    }

    #[test]
    fn package_container_from_resolve_metadata() {
        use crate::package_source::PackageSourceId;
        use crate::resolution::ResolvePackageMetadata;

        let id_registry: PackageId = "ext/util@1.0.0".parse().unwrap();
        let id_workspace: PackageId = "team/foo@1.0.0".parse().unwrap();

        let mut resolve = Resolve::default();
        resolve.metadata.insert(
            id_registry.clone(),
            ResolvePackageMetadata {
                realm: Realm::Shared,
                origin_realm: Realm::Shared,
                source_registry: PackageSourceId::DefaultRegistry,
                is_workspace_member: false,
            },
        );
        resolve.metadata.insert(
            id_workspace.clone(),
            ResolvePackageMetadata {
                realm: Realm::Shared,
                origin_realm: Realm::Shared,
                source_registry: PackageSourceId::Path("/ws/modules/foo".into()),
                is_workspace_member: true,
            },
        );

        assert_eq!(
            package_container(&resolve, &id_registry),
            PackageContainer::Index
        );
        assert_eq!(
            package_container(&resolve, &id_workspace),
            PackageContainer::Workspace
        );
    }

    // =====================================================================
    // Workspace member installation tests
    // =====================================================================

    mod workspace_install_tests {
        use super::*;
        use crate::package_source::{InMemoryRegistry, PackageSourceId};
        use crate::resolution::ResolvePackageMetadata;

        /// Build a Resolve that resembles a workspace with the given members
        /// and optional registry deps between them.
        struct ResolveBuilder {
            resolve: Resolve,
        }

        impl ResolveBuilder {
            fn new() -> Self {
                Self {
                    resolve: Resolve::default(),
                }
            }

            fn add_workspace_member(
                mut self,
                id_str: &str,
                realm: Realm,
                member_dir: &Path,
            ) -> Self {
                let id: PackageId = id_str.parse().unwrap();
                self.resolve.activated.insert(id.clone());
                self.resolve.metadata.insert(
                    id,
                    ResolvePackageMetadata {
                        realm,
                        origin_realm: realm,
                        source_registry: PackageSourceId::Path(member_dir.to_path_buf()),
                        is_workspace_member: true,
                    },
                );
                self
            }

            fn add_registry_package(mut self, id_str: &str, realm: Realm) -> Self {
                let id: PackageId = id_str.parse().unwrap();
                self.resolve.activated.insert(id.clone());
                self.resolve.metadata.insert(
                    id,
                    ResolvePackageMetadata {
                        realm,
                        origin_realm: realm,
                        source_registry: PackageSourceId::DefaultRegistry,
                        is_workspace_member: false,
                    },
                );
                self
            }

            fn add_shared_dep(
                mut self,
                source: &str,
                alias: &str,
                target: &str,
            ) -> Self {
                let src: PackageId = source.parse().unwrap();
                let tgt: PackageId = target.parse().unwrap();
                self.resolve
                    .shared_dependencies
                    .entry(src)
                    .or_default()
                    .insert(alias.to_owned(), tgt);
                self
            }

            fn add_server_dep(
                mut self,
                source: &str,
                alias: &str,
                target: &str,
            ) -> Self {
                let src: PackageId = source.parse().unwrap();
                let tgt: PackageId = target.parse().unwrap();
                self.resolve
                    .server_dependencies
                    .entry(src)
                    .or_default()
                    .insert(alias.to_owned(), tgt);
                self
            }

            fn build(self) -> Resolve {
                self.resolve
            }
        }

        fn empty_sources() -> PackageSourceMap {
            PackageSourceMap::new(Box::new(InMemoryRegistry::new().source()))
        }

        #[test]
        fn workspace_project_json_created_with_correct_path() {
            let tmp = tempfile::TempDir::new().unwrap();
            let ws_root = tmp.path();

            let member_dir = ws_root.join("modules/foo");
            fs::create_dir_all(&member_dir).unwrap();
            let member_dir = member_dir.canonicalize().unwrap();

            let ctx = InstallationContext::new(ws_root, None, None);
            let id: PackageId = "team/foo@1.0.0".parse().unwrap();
            ctx.write_workspace_project_json(&id, Realm::Shared, &member_dir)
                .unwrap();

            let project_json_path = ws_root
                .join("Packages/_Workspace/team_foo@1.0.0/foo/default.project.json");
            assert!(project_json_path.exists());

            let content: serde_json::Value =
                serde_json::from_str(&fs::read_to_string(&project_json_path).unwrap())
                    .unwrap();
            assert_eq!(content["name"], "foo");

            let path_value = content["tree"]["$path"].as_str().unwrap();
            assert!(
                !path_value.is_empty(),
                "path should be non-empty"
            );

            // The $path should resolve back to the member directory.
            let json_parent = project_json_path.parent().unwrap();
            let resolved = json_parent.join(path_value).canonicalize().unwrap();
            assert_eq!(resolved, member_dir);
        }

        #[test]
        fn workspace_project_json_in_server_realm() {
            let tmp = tempfile::TempDir::new().unwrap();
            let ws_root = tmp.path();

            let member_dir = ws_root.join("modules/svc");
            fs::create_dir_all(&member_dir).unwrap();
            let member_dir = member_dir.canonicalize().unwrap();

            let ctx = InstallationContext::new(ws_root, None, None);
            let id: PackageId = "team/svc@1.0.0".parse().unwrap();
            ctx.write_workspace_project_json(&id, Realm::Server, &member_dir)
                .unwrap();

            let project_json_path = ws_root
                .join("ServerPackages/_Workspace/team_svc@1.0.0/svc/default.project.json");
            assert!(project_json_path.exists());

            let content: serde_json::Value =
                serde_json::from_str(&fs::read_to_string(&project_json_path).unwrap())
                    .unwrap();
            assert_eq!(content["name"], "svc");

            let json_parent = project_json_path.parent().unwrap();
            let resolved = json_parent
                .join(content["tree"]["$path"].as_str().unwrap())
                .canonicalize()
                .unwrap();
            assert_eq!(resolved, member_dir);
        }

        #[test]
        fn workspace_members_get_workspace_dir_not_index() {
            let tmp = tempfile::TempDir::new().unwrap();
            let ws_root = tmp.path();

            let foo_dir = ws_root.join("modules/foo");
            let bar_dir = ws_root.join("modules/bar");
            fs::create_dir_all(&foo_dir).unwrap();
            fs::create_dir_all(&bar_dir).unwrap();
            let foo_dir = foo_dir.canonicalize().unwrap();
            let bar_dir = bar_dir.canonicalize().unwrap();

            let resolve = ResolveBuilder::new()
                .add_workspace_member("team/foo@1.0.0", Realm::Shared, &foo_dir)
                .add_workspace_member("team/bar@1.0.0", Realm::Shared, &bar_dir)
                .add_shared_dep("team/foo@1.0.0", "Bar", "team/bar@1.0.0")
                .build();

            let foo_id: PackageId = "team/foo@1.0.0".parse().unwrap();
            let bar_id: PackageId = "team/bar@1.0.0".parse().unwrap();
            let root_ids = BTreeSet::from([foo_id.clone(), bar_id.clone()]);

            let ctx = InstallationContext::new(ws_root, None, None);
            ctx.install(empty_sources(), root_ids, resolve).unwrap();

            // _Workspace directories should exist for both members
            assert!(ws_root
                .join("Packages/_Workspace/team_foo@1.0.0/foo/default.project.json")
                .exists());
            assert!(ws_root
                .join("Packages/_Workspace/team_bar@1.0.0/bar/default.project.json")
                .exists());

            // _Index should NOT have these workspace packages
            assert!(!ws_root
                .join("Packages/_Index/team_foo@1.0.0")
                .exists());
            assert!(!ws_root
                .join("Packages/_Index/team_bar@1.0.0")
                .exists());
        }

        #[test]
        fn workspace_root_links_point_to_workspace_container() {
            let tmp = tempfile::TempDir::new().unwrap();
            let ws_root = tmp.path();

            let foo_dir = ws_root.join("modules/foo");
            let bar_dir = ws_root.join("modules/bar");
            fs::create_dir_all(&foo_dir).unwrap();
            fs::create_dir_all(&bar_dir).unwrap();
            let foo_dir = foo_dir.canonicalize().unwrap();
            let bar_dir = bar_dir.canonicalize().unwrap();

            let resolve = ResolveBuilder::new()
                .add_workspace_member("team/foo@1.0.0", Realm::Shared, &foo_dir)
                .add_workspace_member("team/bar@1.0.0", Realm::Shared, &bar_dir)
                .add_shared_dep("team/foo@1.0.0", "Bar", "team/bar@1.0.0")
                .build();

            let foo_id: PackageId = "team/foo@1.0.0".parse().unwrap();
            let bar_id: PackageId = "team/bar@1.0.0".parse().unwrap();
            let root_ids = BTreeSet::from([foo_id.clone(), bar_id.clone()]);

            let ctx = InstallationContext::new(ws_root, None, None);
            ctx.install(empty_sources(), root_ids, resolve).unwrap();

            // Foo depends on Bar, so there should be a root link Packages/Bar.lua
            let root_link = ws_root.join("Packages/Bar.lua");
            assert!(root_link.exists(), "root link for Bar should exist");
            let content = fs::read_to_string(&root_link).unwrap();
            assert!(
                content.contains("_Workspace"),
                "root link should point into _Workspace, got: {}",
                content
            );
            assert!(content.contains("team_bar@1.0.0"));
        }

        #[test]
        fn workspace_sibling_links_written_for_members() {
            let tmp = tempfile::TempDir::new().unwrap();
            let ws_root = tmp.path();

            let foo_dir = ws_root.join("modules/foo");
            let bar_dir = ws_root.join("modules/bar");
            fs::create_dir_all(&foo_dir).unwrap();
            fs::create_dir_all(&bar_dir).unwrap();
            let foo_dir = foo_dir.canonicalize().unwrap();
            let bar_dir = bar_dir.canonicalize().unwrap();

            let resolve = ResolveBuilder::new()
                .add_workspace_member("team/foo@1.0.0", Realm::Shared, &foo_dir)
                .add_workspace_member("team/bar@1.0.0", Realm::Shared, &bar_dir)
                .add_shared_dep("team/foo@1.0.0", "Bar", "team/bar@1.0.0")
                .build();

            let foo_id: PackageId = "team/foo@1.0.0".parse().unwrap();
            let bar_id: PackageId = "team/bar@1.0.0".parse().unwrap();
            let root_ids = BTreeSet::from([foo_id.clone(), bar_id.clone()]);

            let ctx = InstallationContext::new(ws_root, None, None);
            ctx.install(empty_sources(), root_ids, resolve).unwrap();

            // Foo's _Workspace entry should have a sibling link to Bar
            let sibling_link = ws_root
                .join("Packages/_Workspace/team_foo@1.0.0/Bar.lua");
            assert!(
                sibling_link.exists(),
                "sibling link for Bar in foo's workspace entry should exist"
            );
            let content = fs::read_to_string(&sibling_link).unwrap();
            assert!(
                content.contains("script.Parent.Parent"),
                "sibling link should use script.Parent.Parent navigation"
            );
            assert!(content.contains("team_bar@1.0.0"));
        }

        #[test]
        fn workspace_cross_container_links() {
            let ctx = InstallationContext::new(Path::new("/project"), None, None);
            let util_id: PackageId = "ext/util@1.0.0".parse().unwrap();

            let link = ctx.link_sibling(
                &util_id,
                PackageContainer::Workspace,
                PackageContainer::Index,
            );
            assert!(
                link.contains("script.Parent.Parent.Parent._Index"),
                "cross-container link should go up to realm root then into _Index: {}",
                link
            );
        }

        #[test]
        fn single_package_backward_compat_no_workspace_dir() {
            use crate::test_package::PackageBuilder;

            let tmp = tempfile::TempDir::new().unwrap();
            let ws_root = tmp.path();

            let root_dir = ws_root.canonicalize().unwrap();
            let resolve = ResolveBuilder::new()
                .add_workspace_member("biff/mypackage@1.0.0", Realm::Shared, &root_dir)
                .add_registry_package("ext/dep@1.0.0", Realm::Shared)
                .add_shared_dep(
                    "biff/mypackage@1.0.0",
                    "Dep",
                    "ext/dep@1.0.0",
                )
                .build();

            let root_id: PackageId = "biff/mypackage@1.0.0".parse().unwrap();
            let root_ids = BTreeSet::from([root_id.clone()]);

            let registry = InMemoryRegistry::new();
            registry.publish(PackageBuilder::new("ext/dep@1.0.0"));
            let sources = PackageSourceMap::new(Box::new(registry.source()));

            let ctx = InstallationContext::new(ws_root, None, None);
            ctx.install(sources, root_ids, resolve).unwrap();

            // Root link should exist for the registry dep
            assert!(ws_root.join("Packages/Dep.lua").exists());

            // _Index should have the registry package
            assert!(ws_root.join("Packages/_Index/ext_dep@1.0.0").exists());

            // No _Workspace directory should be created for the sole root
            assert!(
                !ws_root.join("Packages/_Workspace").exists(),
                "single-package project should not create _Workspace"
            );
        }

        #[test]
        fn workspace_cross_realm_member_links() {
            let tmp = tempfile::TempDir::new().unwrap();
            let ws_root = tmp.path();

            let svc_dir = ws_root.join("modules/svc");
            let lib_dir = ws_root.join("modules/shared-lib");
            fs::create_dir_all(&svc_dir).unwrap();
            fs::create_dir_all(&lib_dir).unwrap();
            let svc_dir = svc_dir.canonicalize().unwrap();
            let lib_dir = lib_dir.canonicalize().unwrap();

            let resolve = ResolveBuilder::new()
                .add_workspace_member("team/svc@1.0.0", Realm::Server, &svc_dir)
                .add_workspace_member("team/shared-lib@1.0.0", Realm::Shared, &lib_dir)
                .add_server_dep(
                    "team/svc@1.0.0",
                    "SharedLib",
                    "team/shared-lib@1.0.0",
                )
                .build();

            let svc_id: PackageId = "team/svc@1.0.0".parse().unwrap();
            let lib_id: PackageId = "team/shared-lib@1.0.0".parse().unwrap();
            let root_ids = BTreeSet::from([svc_id.clone(), lib_id.clone()]);

            let ctx = InstallationContext::new(
                ws_root,
                Some("game.ReplicatedStorage.Packages".to_string()),
                Some("game.ServerScriptService.Packages".to_string()),
            );
            ctx.install(empty_sources(), root_ids, resolve).unwrap();

            // svc is server realm, shared-lib is shared realm
            assert!(ws_root
                .join("ServerPackages/_Workspace/team_svc@1.0.0/svc/default.project.json")
                .exists());
            assert!(ws_root
                .join("Packages/_Workspace/team_shared-lib@1.0.0/shared-lib/default.project.json")
                .exists());

            // svc depends on shared-lib (cross-realm) - the link should use
            // the absolute DataModel path.
            let cross_realm_link = ws_root
                .join("ServerPackages/_Workspace/team_svc@1.0.0/SharedLib.lua");
            assert!(
                cross_realm_link.exists(),
                "cross-realm link should exist: {}",
                cross_realm_link.display()
            );
            let content = fs::read_to_string(&cross_realm_link).unwrap();
            assert!(
                content.contains("game.ReplicatedStorage.Packages._Workspace"),
                "cross-realm link should reference shared Packages._Workspace: {}",
                content
            );
        }

        #[test]
        fn workspace_project_json_path_resolves_correctly_with_deep_nesting() {
            let tmp = tempfile::TempDir::new().unwrap();
            let ws_root = tmp.path();

            let member_dir = ws_root.join("deep/nested/path/member");
            fs::create_dir_all(&member_dir).unwrap();
            let member_dir = member_dir.canonicalize().unwrap();

            let ctx = InstallationContext::new(ws_root, None, None);
            let id: PackageId = "team/member@2.0.0".parse().unwrap();
            ctx.write_workspace_project_json(&id, Realm::Shared, &member_dir)
                .unwrap();

            let json_path = ws_root
                .join("Packages/_Workspace/team_member@2.0.0/member/default.project.json");
            assert!(json_path.exists());

            let content: serde_json::Value =
                serde_json::from_str(&fs::read_to_string(&json_path).unwrap()).unwrap();
            let path_str = content["tree"]["$path"].as_str().unwrap();

            // Verify the path resolves correctly
            let resolved = json_path.parent().unwrap().join(path_str).canonicalize().unwrap();
            assert_eq!(resolved, member_dir);
        }
    }
}
