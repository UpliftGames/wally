use std::collections::BTreeMap;
use std::path::Path;

use anyhow::Context;
use semver::Version;
use serde::{Deserialize, Serialize};

use crate::package_id::PackageId;
use crate::package_name::PackageName;
use crate::package_req::PackageReq;

pub const MANIFEST_FILE_NAME: &str = "wally.toml";

/// The contents of a `wally.toml` file, which defines a package.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Manifest {
    pub package: Package,

    #[serde(default)]
    pub place: PlaceInfo,

    #[serde(default)]
    pub dependencies: BTreeMap<String, PackageReq>,

    #[serde(default)]
    pub server_dependencies: BTreeMap<String, PackageReq>,

    #[serde(default)]
    pub dev_dependencies: BTreeMap<String, PackageReq>,
}

impl Manifest {
    /// Load a manifest from a project directory containing a `wally.toml` file.
    pub fn load(dir: &Path) -> anyhow::Result<Self> {
        let file_path = dir.join(MANIFEST_FILE_NAME);

        let content = fs_err::read_to_string(&file_path)?;
        let manifest: Manifest = toml::from_str(&content)
            .with_context(|| format!("failed to parse manifest at path {}", file_path.display()))?;

        Ok(manifest)
    }

    pub fn from_slice(slice: &[u8]) -> anyhow::Result<Self> {
        let manifest: Manifest =
            toml::from_slice(slice).with_context(|| format!("failed to parse manifest"))?;

        Ok(manifest)
    }

    pub fn package_id(&self) -> PackageId {
        PackageId::new(self.package.name.clone(), self.package.version.clone())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Package {
    /// The scope and name of the package.
    ///
    /// Example: `lpghatguy/asink`.
    pub name: PackageName,

    /// The current version of the package.
    ///
    /// Example: `1.0.0`
    pub version: Version,

    /// The registry that this package should pull its dependencies from.
    ///
    /// Example: `https://github.com/UpliftGames/wally-test-index`
    pub registry: String,

    /// The realms (`shared`, `server`, etc) that this package can be used in.
    ///
    /// Packages in the `shared` realm can only depend on other `shared`
    /// packages. Packages in the `server` realm can depend on any other
    /// package.
    ///
    /// Example: `shared`, `server`
    pub realm: Realm,

    /// A short description of the package.
    ///
    /// Example: `A game about adopting things.`
    pub description: Option<String>,

    /// An SPDX license specifier for the package.
    ///
    /// Example: `MIT OR Apache-2.0`
    pub license: Option<String>,

    /// A list of the package's authors.
    ///
    /// Example: ["Biff Lumfer <biff@playadopt.me>"]
    #[serde(default)]
    pub authors: Vec<String>,

    /// A list of paths to include in the package. Glob patterns are supported.
    ///
    /// By default all directories and files are included except files generated
    /// by wally and hidden files/directories. If include is specified then only
    /// files matching patterns in the include list will be included.
    ///
    /// If include is unspecified and a .gitignore file exists then those patterns
    /// will be respected and wally will also ignore those files.
    ///
    /// Example: ["/src", "*.lua"]
    #[serde(default)]
    pub include: Vec<String>,

    /// A list of paths to exclude from the package. Glob patterns are supported.
    ///
    /// By default files generated by wally and hidden files/directories will be
    /// excluded. If a .gitignore file exists and include is unspecified then
    /// those patterns will be respected and wally will also ignore those files.
    /// Patterns in exclude will be excluded in addition to those patterns in the
    /// .gitignore.
    ///
    /// Example: ["/Packages", "/node_modules"]
    #[serde(default)]
    pub exclude: Vec<String>,

    /// Indicates whether the package can be published or not.
    ///
    /// Example: true
    #[serde(default)]
    pub private: bool,

    /// URL of the package homepage.
    ///
    /// Example: "https://github.com/sleitnick/knit"
    #[serde(default)]
    pub homepage: Option<String>,

    /// URL of the package source repository.
    ///
    /// Example: "https://github.com/Sleitnick/Knit.git"
    #[serde(default)]
    pub repository: Option<String>,
}

// Metadata we require when this manifest will be used to generate package folders
// This information can be present in any package but is only used in the root package
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct PlaceInfo {
    /// Where the shared packages folder is located in the Roblox Datamodel
    ///
    /// Example: `game.ReplicatedStorage.Packages`
    #[serde(default)]
    pub shared_packages: Option<String>,

    /// Where the server packages folder is located in the Roblox Datamodel
    ///
    /// Example: `game.ServerScriptStorage.Packages`
    #[serde(default)]
    pub server_packages: Option<String>,
}

impl Default for PlaceInfo {
    fn default() -> Self {
        Self {
            shared_packages: None,
            server_packages: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Realm {
    Server,
    Shared,
    Dev,
}

impl Realm {
    pub fn is_dependency_valid(dep_type: Self, dep_realm: Self) -> bool {
        use Realm::*;

        matches!(
            (dep_type, dep_realm),
            (Server, _) | (Shared, Shared) | (Dev, _)
        )
    }
}
