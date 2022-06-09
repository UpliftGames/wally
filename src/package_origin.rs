use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::package_source::PackageSourceId;

// I hate you with all with a burning passion.

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PackageOrigin {
    Path(PathBuf),
    Git(String),
    Registry(PackageSourceId),
}
