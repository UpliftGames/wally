use std::{
    collections::BTreeMap, fmt::Display, io, path::{Path, PathBuf}, time::Duration
};

use anyhow::{bail, format_err};
use crossterm::style::{Color, SetForegroundColor};
use fs_err as fs;
use indicatif::{ProgressBar, ProgressStyle};
use indoc::{formatdoc, indoc};

use crate::{
    extract_types::{extract_types, ExtractTypesResult},
    manifest::Realm,
    package_contents::PackageContents,
    package_id::PackageId,
    package_source::{PackageSourceMap, PackageSourceProvider},
    resolution::Resolve,
};

#[derive(Clone)]
pub struct InstallationContext {
    shared_dir: PathBuf,
    shared_index_dir: PathBuf,
    shared_path: Option<String>,
    server_dir: PathBuf,
    server_index_dir: PathBuf,
    server_path: Option<String>,
    dev_dir: PathBuf,
    dev_index_dir: PathBuf,
}

type PackageTypeExports = BTreeMap<PackageId, ExtractTypesResult>;

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

        Self {
            shared_dir,
            shared_index_dir,
            shared_path,
            server_dir,
            server_index_dir,
            server_path,
            dev_dir,
            dev_index_dir,
        }
    }

    /// Delete the existing index, if it exists.
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

    /// Install all packages from the given `Resolve` into the package that this
    /// `InstallationContext` was built for.
    pub fn install(
        self,
        sources: PackageSourceMap,
        root_package_id: PackageId,
        resolved: Resolve,
    ) -> anyhow::Result<()> {
        let mut handles = Vec::new();
        let resolved_copy = resolved.clone();
        let bar = ProgressBar::new((resolved_copy.activated.len() - 1) as u64).with_style(
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

        for package_id in &resolved_copy.activated {
            // Shadow because the thread will need to take ownership of this value.
            let package_id = package_id.clone();
            if package_id != root_package_id {
                log::debug!("Downloading package {}...", package_id);

                let metadata = resolved.metadata.get(&package_id).unwrap();
                let package_realm = metadata.origin_realm;
                let source_registry = resolved_copy.metadata[&package_id].source_registry.clone();
                let source_copy = sources.clone();
                let context = self.clone();
                let b = bar.clone();

                let handle = runtime.spawn_blocking(move || {
                    let package_source = source_copy.get(&source_registry).unwrap();
                    let contents = package_source.download_package(&package_id)?;
                    b.println(format!(
                        "{} Downloaded {}{}",
                        SetForegroundColor(Color::DarkGreen),
                        SetForegroundColor(Color::Reset),
                        package_id,
                    ));
                    b.inc(1);

                    let write_result =
                        context.write_contents(&package_id, &contents, package_realm);
                    write_result.map(|path| {
                        (package_id, extract_types(&path))
                    })
                });

                handles.push(handle);
            }
        }

        let num_packages = handles.len();
        let mut types_for_package = PackageTypeExports::new();
        for handle in handles {
            let (package_id, exported_types) = runtime
                .block_on(handle)
                .expect("Package failed to be installed.")?;

            types_for_package.insert(package_id, exported_types);
        }

        for package_id in &resolved_copy.activated {
            log::debug!("Installing package {}...", package_id);

            let shared_deps = resolved.shared_dependencies.get(&package_id);
            let server_deps = resolved.server_dependencies.get(&package_id);
            let dev_deps = resolved.dev_dependencies.get(&package_id);

            // Then 3), run these loops, passing in the registry object.
            // We do not need to install the root package, but we should create
            // package links for its dependencies.
            if *package_id == root_package_id {
                if let Some(deps) = shared_deps {
                    self.write_root_package_links(Realm::Shared, deps, &resolved, &types_for_package)?;
                }

                if let Some(deps) = server_deps {
                    self.write_root_package_links(Realm::Server, deps, &resolved, &types_for_package)?;
                }

                if let Some(deps) = dev_deps {
                    self.write_root_package_links(Realm::Dev, deps, &resolved, &types_for_package)?;
                }
            } else {
                let metadata = resolved.metadata.get(&package_id).unwrap();
                let package_realm = metadata.origin_realm;

                if let Some(deps) = shared_deps {
                    self.write_package_links(&package_id, package_realm, deps, &resolved, &types_for_package)?;
                }

                if let Some(deps) = server_deps {
                    self.write_package_links(&package_id, package_realm, deps, &resolved, &types_for_package)?;
                }

                if let Some(deps) = dev_deps {
                    self.write_package_links(&package_id, package_realm, deps, &resolved, &types_for_package)?;
                }
            }
        }

        bar.finish_and_clear();
        log::info!("Downloaded {} packages!", num_packages);

        Ok(())
    }

    /// Contents of a package-to-package link within the same index.
    fn link_sibling_same_index(&self, id: &PackageId, exports: &ExtractTypesResult) -> String {
        // TODO: Here, pass and write set of types
        if exports.is_empty() {
            formatdoc! {r#"
                return require(script.Parent.Parent["{full_name}"]["{short_name}"])
                "#,
                full_name = package_id_file_name(id),
                short_name = id.name().name()
            }
        } else {
            formatdoc! {r#"
                local MODULE = require(script.Parent.Parent["{full_name}"]["{short_name}"])
                {exports_string}
                return MODULE
                "#,
                full_name = package_id_file_name(id),
                short_name = id.name().name(),
                exports_string = exports.format_forwarding_statements("MODULE")
            }
        }

    }

    /// Contents of a root-to-package link within the same index.
    fn link_root_same_index(&self, id: &PackageId, exports: &ExtractTypesResult) -> String {
        if exports.is_empty() {
            formatdoc! {r#"
                return require(script.Parent._Index["{full_name}"]["{short_name}"])
                "#,
                full_name = package_id_file_name(id),
                short_name = id.name().name()
            }
        } else {
            formatdoc! {r#"
                local MODULE = require(script.Parent._Index["{full_name}"]["{short_name}"])
                {exports_string}
                return MODULE
                "#,
                full_name = package_id_file_name(id),
                short_name = id.name().name(),
                exports_string = exports.format_forwarding_statements("MODULE")
            }
        }
    }

    /// Contents of a link into the shared index from outside the shared index.
    fn link_shared_index(&self, id: &PackageId, exports: &ExtractTypesResult) -> anyhow::Result<String> {
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

        let contents = if exports.is_empty() {
            formatdoc! {r#"
                return require({packages}._Index["{full_name}"]["{short_name}"])
                "#,
                packages = shared_path,
                full_name = package_id_file_name(id),
                short_name = id.name().name()
            }
        } else {
            formatdoc! {r#"
                local MODULE = require({packages}._Index["{full_name}"]["{short_name}"])
                {exports_string}
                return MODULE
                "#,
                packages = shared_path,
                full_name = package_id_file_name(id),
                short_name = id.name().name(),
                exports_string = exports.format_forwarding_statements("MODULE")
            }
        };

        Ok(contents)
    }

    /// Contents of a link into the server index from outside the server index.
    fn link_server_index(&self, id: &PackageId, exports: &ExtractTypesResult) -> anyhow::Result<String> {
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

        let contents = if exports.is_empty() {
            formatdoc! {r#"
                return require({packages}._Index["{full_name}"]["{short_name}"])
                "#,
                packages = server_path,
                full_name = package_id_file_name(id),
                short_name = id.name().name()
            }
        } else {
            formatdoc! {r#"
                local MODULE = require({packages}._Index["{full_name}"]["{short_name}"])
                {exports_string}
                return MODULE
                "#,
                packages = server_path,
                full_name = package_id_file_name(id),
                short_name = id.name().name(),
                exports_string = exports.format_forwarding_statements("MODULE")
            }
        };

        Ok(contents)
    }

    fn write_root_package_links<'a, K: Display>(
        &self,
        root_realm: Realm,
        dependencies: impl IntoIterator<Item = (K, &'a PackageId)>,
        resolved: &Resolve,
        types: &PackageTypeExports
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
            let dependencies_realm = resolved.metadata.get(dep_package_id).unwrap().origin_realm;
            let path = base_path.join(format!("{}.lua", dep_name));
            let types_for_dep = types.get(dep_package_id).unwrap();

            let contents = match (root_realm, dependencies_realm) {
                (source, dest) if source == dest => self.link_root_same_index(dep_package_id, types_for_dep),
                (_, Realm::Server) => self.link_server_index(dep_package_id, types_for_dep)?,
                (_, Realm::Shared) => self.link_shared_index(dep_package_id, types_for_dep)?,
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
        types: &PackageTypeExports
    ) -> anyhow::Result<()> {
        log::debug!("Writing package links for {}", package_id);

        let mut base_path = match package_realm {
            Realm::Shared => self.shared_index_dir.clone(),
            Realm::Server => self.server_index_dir.clone(),
            Realm::Dev => self.dev_index_dir.clone(),
        };

        base_path.push(package_id_file_name(package_id));

        log::trace!("Creating directory {}", base_path.display());
        fs::create_dir_all(&base_path)?;

        for (dep_name, dep_package_id) in dependencies {
            let dependencies_realm = resolved.metadata.get(dep_package_id).unwrap().origin_realm;
            let path = base_path.join(format!("{}.lua", dep_name));
            let types_for_dep = types.get(dep_package_id).unwrap();

            let contents = match (package_realm, dependencies_realm) {
                (source, dest) if source == dest => self.link_sibling_same_index(dep_package_id, types_for_dep),
                (_, Realm::Server) => self.link_server_index(dep_package_id, types_for_dep)?,
                (_, Realm::Shared) => self.link_shared_index(dep_package_id, types_for_dep)?,
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
    ) -> anyhow::Result<PathBuf> {
        let mut path = match realm {
            Realm::Shared => self.shared_index_dir.clone(),
            Realm::Server => self.server_index_dir.clone(),
            Realm::Dev => self.dev_index_dir.clone(),
        };

        path.push(package_id_file_name(package_id));
        path.push(package_id.name().name());

        fs::create_dir_all(&path)?;
        contents.unpack_into_path(&path)?;

        Ok(path)
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
