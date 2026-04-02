use std::fmt;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::package_req::PackageReq;

/// Specifies how a dependency should be resolved.
///
/// Variant ordering matters: `#[serde(untagged)]` tries each variant in
/// declaration order. `Registry` (a plain string) must come first so that
/// `"scope/name@version"` is not accidentally consumed by a table variant.
/// `Path` (table with `path` key) comes next, then `Workspace` (table with
/// `workspace` key).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DependencySpec {
    /// A registry dependency: `"scope/name@version"`.
    Registry(PackageReq),
    /// A local path dependency: `{ path = "../sibling" }`.
    Path(PathDependencySpec),
    /// A workspace-inherited dependency: `{ workspace = true }`.
    /// Resolved during workspace loading by replacing with the concrete spec
    /// from `[workspace.dependencies]`. By the time the resolver runs, no
    /// `Workspace` variants remain.
    Workspace { workspace: bool },
}

/// Points to a local directory containing a `wally.toml`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PathDependencySpec {
    pub path: PathBuf,
}

impl DependencySpec {
    /// Returns the inner `PackageReq` if this is a registry dependency.
    pub fn as_registry(&self) -> Option<&PackageReq> {
        match self {
            DependencySpec::Registry(req) => Some(req),
            _ => None,
        }
    }

    /// Unwraps the inner `PackageReq`, panicking if this is not a registry
    /// dependency.
    pub fn expect_registry(&self) -> &PackageReq {
        match self {
            DependencySpec::Registry(req) => req,
            other => panic!("expected Registry dependency, got {:?}", other),
        }
    }

    /// Returns the inner `PathDependencySpec` if this is a path dependency.
    pub fn as_path(&self) -> Option<&PathDependencySpec> {
        match self {
            DependencySpec::Path(spec) => Some(spec),
            _ => None,
        }
    }

    /// Returns `true` if this is a `{ workspace = true }` directive.
    pub fn is_workspace(&self) -> bool {
        matches!(self, DependencySpec::Workspace { workspace: true })
    }
}

impl fmt::Display for DependencySpec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DependencySpec::Registry(req) => write!(f, "{}", req),
            DependencySpec::Path(spec) => write!(f, "path: {}", spec.path.display()),
            DependencySpec::Workspace { workspace } => {
                write!(f, "workspace: {}", workspace)
            }
        }
    }
}

impl From<PackageReq> for DependencySpec {
    fn from(req: PackageReq) -> Self {
        DependencySpec::Registry(req)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // Registry (existing behaviour preserved)
    // -----------------------------------------------------------------------

    #[test]
    fn serde_round_trip_json() {
        let req: PackageReq = "hello/world@1.2.3".parse().unwrap();
        let spec = DependencySpec::Registry(req.clone());

        let json = serde_json::to_string(&spec).unwrap();
        let deserialized: DependencySpec = serde_json::from_str(&json).unwrap();
        assert_eq!(spec, deserialized);

        let req_json = serde_json::to_string(&req).unwrap();
        assert_eq!(json, req_json);
    }

    #[test]
    fn serde_round_trip_toml_value() {
        let req: PackageReq = "biff/minimal@0.1.0".parse().unwrap();
        let spec = DependencySpec::Registry(req.clone());

        let toml_val = toml::Value::try_from(&spec).unwrap();
        let deserialized: DependencySpec = toml_val.try_into().unwrap();
        assert_eq!(spec, deserialized);
    }

    #[test]
    fn deserialize_from_string() {
        let spec: DependencySpec = serde_json::from_str("\"roblox/roact@1.4.2\"").unwrap();
        assert!(spec.as_registry().is_some());
        assert_eq!(spec.expect_registry().name().scope(), "roblox");
        assert_eq!(spec.expect_registry().name().name(), "roact");
    }

    #[test]
    fn display_registry() {
        let req: PackageReq = "hello/world@1.0.0".parse().unwrap();
        let spec = DependencySpec::Registry(req.clone());
        assert_eq!(spec.to_string(), req.to_string());
    }

    #[test]
    fn from_package_req() {
        let req: PackageReq = "foo/bar@2.0.0".parse().unwrap();
        let spec: DependencySpec = req.clone().into();
        assert_eq!(spec.expect_registry(), &req);
    }

    #[test]
    fn manifest_round_trip_toml() {
        use crate::manifest::Manifest;

        let toml_str = r#"
            [package]
            name = "test/my-package"
            version = "1.0.0"
            registry = "https://github.com/UpliftGames/wally-index"
            realm = "shared"

            [dependencies]
            Roact = "roblox/roact@1.4.0"
            Promise = "evaera/promise@3.0.0"

            [server-dependencies]
            DataStore = "sleitnick/datastore2@1.0.0"
        "#;

        let manifest: Manifest = toml::from_str(toml_str).unwrap();

        assert_eq!(manifest.dependencies.len(), 2);
        assert_eq!(manifest.server_dependencies.len(), 1);
        assert_eq!(manifest.dev_dependencies.len(), 0);

        let roact = manifest.dependencies.get("Roact").unwrap();
        assert_eq!(roact.expect_registry().name().scope(), "roblox");
        assert_eq!(roact.expect_registry().name().name(), "roact");

        let datastore = manifest.server_dependencies.get("DataStore").unwrap();
        assert_eq!(datastore.expect_registry().name().scope(), "sleitnick");

        // Round-trip: serialize back to TOML and re-parse
        let serialized = toml::to_string_pretty(&manifest).unwrap();
        let reparsed: Manifest = toml::from_str(&serialized).unwrap();
        assert_eq!(manifest.dependencies.len(), reparsed.dependencies.len());
        assert_eq!(
            manifest.server_dependencies.len(),
            reparsed.server_dependencies.len()
        );
    }

    // -----------------------------------------------------------------------
    // Path dependency
    // -----------------------------------------------------------------------

    #[test]
    fn parse_path_dep_toml() {
        #[derive(serde::Deserialize)]
        struct Helper {
            dep: DependencySpec,
        }

        let h: Helper = toml::from_str(r#"dep = { path = "../foo" }"#).unwrap();
        let path_spec = h.dep.as_path().expect("expected Path variant");
        assert_eq!(path_spec.path, PathBuf::from("../foo"));
    }

    #[test]
    fn serde_round_trip_path_json() {
        let spec = DependencySpec::Path(PathDependencySpec {
            path: PathBuf::from("../sibling"),
        });
        let json = serde_json::to_string(&spec).unwrap();
        let deserialized: DependencySpec = serde_json::from_str(&json).unwrap();
        assert_eq!(spec, deserialized);
    }

    #[test]
    fn serde_round_trip_path_toml() {
        let spec = DependencySpec::Path(PathDependencySpec {
            path: PathBuf::from("../lib"),
        });
        let toml_val = toml::Value::try_from(&spec).unwrap();
        let deserialized: DependencySpec = toml_val.try_into().unwrap();
        assert_eq!(spec, deserialized);
    }

    #[test]
    fn display_path() {
        let spec = DependencySpec::Path(PathDependencySpec {
            path: PathBuf::from("../foo"),
        });
        assert!(spec.to_string().contains("../foo"));
    }

    #[test]
    fn as_path_returns_none_for_registry() {
        let req: PackageReq = "hello/world@1.0.0".parse().unwrap();
        let spec = DependencySpec::Registry(req);
        assert!(spec.as_path().is_none());
    }

    // -----------------------------------------------------------------------
    // Workspace dependency
    // -----------------------------------------------------------------------

    #[test]
    fn parse_workspace_dep_toml() {
        #[derive(serde::Deserialize)]
        struct Helper {
            dep: DependencySpec,
        }

        let h: Helper = toml::from_str(r#"dep = { workspace = true }"#).unwrap();
        assert!(h.dep.is_workspace());
    }

    #[test]
    fn parse_workspace_false_toml() {
        #[derive(serde::Deserialize)]
        struct Helper {
            dep: DependencySpec,
        }

        let h: Helper = toml::from_str(r#"dep = { workspace = false }"#).unwrap();
        assert!(!h.dep.is_workspace());
        assert!(matches!(
            h.dep,
            DependencySpec::Workspace { workspace: false }
        ));
    }

    #[test]
    fn serde_round_trip_workspace_json() {
        let spec = DependencySpec::Workspace { workspace: true };
        let json = serde_json::to_string(&spec).unwrap();
        let deserialized: DependencySpec = serde_json::from_str(&json).unwrap();
        assert_eq!(spec, deserialized);
    }

    #[test]
    fn display_workspace() {
        let spec = DependencySpec::Workspace { workspace: true };
        assert!(spec.to_string().contains("true"));
    }

    #[test]
    fn is_workspace_false_for_non_workspace() {
        let req: PackageReq = "hello/world@1.0.0".parse().unwrap();
        assert!(!DependencySpec::Registry(req).is_workspace());
        assert!(
            !DependencySpec::Path(PathDependencySpec {
                path: PathBuf::from("../foo")
            })
            .is_workspace()
        );
    }

    // -----------------------------------------------------------------------
    // Mixed dep table with all three variants
    // -----------------------------------------------------------------------

    #[test]
    fn mixed_dep_table_toml() {
        use std::collections::BTreeMap;

        let toml_str = r#"
            Roact = "roblox/roact@1.4.0"
            Sibling = { path = "../sibling" }
            Shared = { workspace = true }
        "#;

        let deps: BTreeMap<String, DependencySpec> = toml::from_str(toml_str).unwrap();
        assert_eq!(deps.len(), 3);

        assert!(deps.get("Roact").unwrap().as_registry().is_some());
        assert!(deps.get("Sibling").unwrap().as_path().is_some());
        assert!(deps.get("Shared").unwrap().is_workspace());
    }

    // -----------------------------------------------------------------------
    // Existing string format still works (backward compat)
    // -----------------------------------------------------------------------

    #[test]
    fn existing_string_format_still_parses_as_registry() {
        let toml_val = toml::Value::try_from("scope/name@1.0.0").unwrap();
        let spec: DependencySpec = toml_val.try_into().unwrap();
        assert!(spec.as_registry().is_some());
    }
}
