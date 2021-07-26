use serde::{Deserialize, Serialize};
use url::Url;

use crate::{auth::AuthMode, storage::StorageMode};

#[derive(Deserialize, Serialize)]
pub struct Config {
    /// The URL of the Git repository containing the registry's package index.
    pub index_url: Url,

    /// The token that should be used by the registry to communicate with
    /// GitHub. If not specified, will try to use the machine's Git credential
    /// helper.
    pub github_token: Option<String>,

    /// What kind of authentication is required to access endpoints.
    pub auth: AuthMode,

    /// Which storage backend to use.
    pub storage: StorageMode,
}
