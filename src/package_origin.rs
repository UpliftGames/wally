use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::package_source::PackageSourceId;

/// PackageOrigin is used to track where the package was originally found.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PackageOrigin {
    Path(PathBuf),
    Git(String),
    Registry(PackageSourceId),
}
