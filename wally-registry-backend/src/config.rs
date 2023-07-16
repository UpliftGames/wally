use semver::Version;
use serde::{Deserialize, Serialize};
use url::Url;

use crate::{analytics::AnalyticsMode, auth::AuthMode, storage::StorageMode};

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

    /// The minimum wally cli version required to publish to the registry
    pub minimum_wally_version: Option<Version>,

    /// What analytics backend should be used, currently the only option is Postgres
    pub analytics: Option<AnalyticsMode>,
}
