use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context};
use semver::VersionReq;
use structopt::StructOpt;
use ubyte::ToByteUnit;
use url::Url;
use walkdir::WalkDir;

use crate::dependency_spec::DependencySpec;
use crate::manifest::Manifest;
use crate::package_contents::PackageContents;
use crate::package_index::PackageIndex;
use crate::package_name::PackageName;
use crate::package_req::PackageReq;
use crate::workspace::Workspace;
use crate::{auth::AuthStore, GlobalOptions};

const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Publish this project to a registry.
#[derive(Debug, StructOpt)]
pub struct PublishSubcommand {
    /// Path to the project to publish.
    #[structopt(long = "project-path", default_value = ".")]
    pub project_path: PathBuf,

    /// Auth token to use
    #[structopt(long = "token")]
    pub token: Option<String>,

    /// Name of the workspace member to publish (e.g. "scope/name").
    /// Required when publishing from a workspace.
    #[structopt(long = "package")]
    pub package: Option<PackageName>,
}

impl PublishSubcommand {
    pub fn run(self, global: GlobalOptions) -> anyhow::Result<()> {
        let workspace_root = Workspace::discover_root(&self.project_path)?;
        let workspace = Workspace::load(&workspace_root)?;

        if workspace.is_single_package() && self.package.is_none() {
            return self.publish_single_package(&workspace, global);
        }

        if workspace.is_single_package() && self.package.is_some() {
            bail!(
                "--package flag is only valid in a workspace context, \
                 but this project is a single-package manifest"
            );
        }

        let pkg_name = self.package.as_ref().ok_or_else(|| {
            anyhow::anyhow!(
                "this is a workspace project; use --package <scope/name> \
                 to select which member to publish"
            )
        })?;

        let (member_dir, manifest) = workspace
            .find_member_by_name(pkg_name)
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "no workspace member named '{}' found",
                    pkg_name
                )
            })?;

        if manifest.package.private {
            bail!("Cannot publish private package '{}'.", pkg_name);
        }

        validate_publish_graph(manifest, member_dir, &workspace)?;

        let rewritten =
            rewrite_path_deps_to_registry(manifest, member_dir, &workspace)?;

        let temp_dir = tempfile::tempdir()?;
        copy_dir_contents(member_dir, temp_dir.path())?;
        let rewritten_toml = toml::to_string_pretty(&rewritten)?;
        fs_err::write(temp_dir.path().join("wally.toml"), rewritten_toml)?;

        let contents = PackageContents::pack_from_path(temp_dir.path())?;

        self.do_publish(&rewritten, contents, global)
    }

    fn publish_single_package(
        &self,
        workspace: &Workspace,
        global: GlobalOptions,
    ) -> anyhow::Result<()> {
        let (_, manifest) = workspace
            .members()
            .iter()
            .next()
            .expect("single-package workspace has one member");

        if manifest.package.private {
            bail!("Cannot publish private package.");
        }

        let contents = PackageContents::pack_from_path(&self.project_path)?;
        self.do_publish(manifest, contents, global)
    }

    fn do_publish(
        &self,
        manifest: &Manifest,
        contents: PackageContents,
        global: GlobalOptions,
    ) -> anyhow::Result<()> {
        let index_url = if global.test_registry {
            let index_path = Path::new(&manifest.package.registry)
                .join("index")
                .canonicalize()?;

            Url::from_directory_path(index_path).unwrap()
        } else {
            Url::parse(&manifest.package.registry)?
        };

        let package_index = if global.use_temp_index {
            PackageIndex::new_temp(&index_url, None)?
        } else {
            PackageIndex::new(&index_url, None)?
        };

        let api = package_index.config()?.api;

        if contents.data().len() > 2.mebibytes() {
            bail!("Package size exceeds 2MB. Reduce package size and try again.");
        }

        let auth = match &self.token {
            Some(token) => token.clone(),
            None => AuthStore::get_token(api.as_str())?
                .with_context(|| "Authentication is required to publish, use `wally login`")?,
        };

        println!(
            "Publishing {} to {}",
            manifest.package_id(),
            package_index.url()
        );

        if let Some(token) = &global.check_token {
            assert!(token.eq(&auth));
            return Ok(());
        }

        let client = reqwest::blocking::Client::new();
        let response = client
            .post(api.join("/v1/publish")?)
            .header("accept", "application/json")
            .header("Wally-Version", VERSION)
            .bearer_auth(auth)
            .body(contents.data().to_owned())
            .send()?;

        if response.status().is_success() {
            println!("Package published successfully!");
        } else {
            println!("Error: {}", response.status());
            println!("{}", response.text()?);
        }

        Ok(())
    }
}

/// Rewrite all `DependencySpec::Path` entries to `DependencySpec::Registry`
/// pinned to the target member's exact version. The resulting manifest can be
/// published as a standard Wally package.
fn rewrite_path_deps_to_registry(
    manifest: &Manifest,
    member_dir: &Path,
    workspace: &Workspace,
) -> anyhow::Result<Manifest> {
    let mut rewritten = manifest.clone();
    rewrite_dep_map(&mut rewritten.dependencies, member_dir, workspace)
        .context("failed to rewrite [dependencies]")?;
    rewrite_dep_map(&mut rewritten.server_dependencies, member_dir, workspace)
        .context("failed to rewrite [server-dependencies]")?;
    rewrite_dep_map(&mut rewritten.dev_dependencies, member_dir, workspace)
        .context("failed to rewrite [dev-dependencies]")?;
    Ok(rewritten)
}

fn rewrite_dep_map(
    deps: &mut BTreeMap<String, DependencySpec>,
    member_dir: &Path,
    workspace: &Workspace,
) -> anyhow::Result<()> {
    for (alias, spec) in deps.iter_mut() {
        if let DependencySpec::Path(path_spec) = spec {
            let target_dir = member_dir.join(&path_spec.path);
            let target_dir = target_dir
                .canonicalize()
                .with_context(|| {
                    format!(
                        "path dependency '{}' points to {} which could not be resolved",
                        alias,
                        target_dir.display()
                    )
                })?;

            let target_manifest = workspace
                .get_member_at_path(&target_dir)
                .ok_or_else(|| {
                    anyhow::anyhow!(
                        "path dependency '{}' at {} is not a workspace member",
                        alias,
                        target_dir.display()
                    )
                })?;

            let exact_req = VersionReq::exact(&target_manifest.package.version);
            *spec = DependencySpec::Registry(PackageReq::new(
                target_manifest.package.name.clone(),
                exact_req,
            ));
        }
    }
    Ok(())
}

/// Validate that a publishable member does not depend on any private members
/// via path dependencies.
fn validate_publish_graph(
    manifest: &Manifest,
    member_dir: &Path,
    workspace: &Workspace,
) -> anyhow::Result<()> {
    let all_deps = manifest
        .dependencies
        .iter()
        .chain(manifest.server_dependencies.iter())
        .chain(manifest.dev_dependencies.iter());

    for (alias, spec) in all_deps {
        if let DependencySpec::Path(path_spec) = spec {
            let target_dir = member_dir.join(&path_spec.path);
            if let Ok(target_dir) = target_dir.canonicalize() {
                if let Some(target) = workspace.get_member_at_path(&target_dir) {
                    if target.package.private {
                        bail!(
                            "cannot publish '{}': it depends on private member '{}' (via '{}')",
                            manifest.package.name,
                            target.package.name,
                            alias,
                        );
                    }
                }
            }
        }
    }
    Ok(())
}

/// Copy the contents of a directory into another directory.
fn copy_dir_contents(from: &Path, into: &Path) -> anyhow::Result<()> {
    let source = WalkDir::new(from).min_depth(1).follow_links(true);

    for entry in source {
        let entry = entry?;
        let relative_path = entry.path().strip_prefix(from).unwrap();
        let dest_path = into.join(relative_path);

        if entry.file_type().is_dir() {
            fs_err::create_dir(&dest_path)?;
        } else {
            fs_err::copy(entry.path(), dest_path)?;
        }
    }

    Ok(())
}
