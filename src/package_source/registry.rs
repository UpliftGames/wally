use std::io::Read;
use std::sync::Arc;

use anyhow::bail;
use once_cell::unsync::OnceCell;
use reqwest::{blocking::Client, header::AUTHORIZATION};
use url::Url;

use crate::auth::AuthStore;
use crate::manifest::Manifest;
use crate::package_id::PackageId;
use crate::package_index::PackageIndex;
use crate::package_req::PackageReq;
use crate::package_source::{PackageContents, PackageSource};

use super::PackageSourceId;

const VERSION: &str = env!("CARGO_PKG_VERSION");

pub struct Registry {
    index_url: Url,
    auth_token: OnceCell<Option<Arc<str>>>,
    index: OnceCell<PackageIndex>,
    client: Client,
}

impl Registry {
    /// Create a `Registry` from a registry spec, which usually comes from the
    /// `registry` field of a package manifest.
    pub fn from_registry_spec(spec: &str) -> anyhow::Result<Self> {
        let index_url = Url::parse(spec)?;

        Ok(Self {
            index_url,
            auth_token: OnceCell::new(),
            index: OnceCell::new(),
            client: Client::new(),
        })
    }

    fn auth_token(&self) -> anyhow::Result<Option<Arc<str>>> {
        self.auth_token
            .get_or_try_init(|| {
                let store = AuthStore::load()?;
                let token = store.tokens.get(self.api_url()?.as_str());
                match token {
                    Some(token) => Ok(Some(Arc::from(token.as_str()))),
                    None => Ok(None),
                }
            })
            .map(|token| token.clone())
    }

    fn index(&self) -> anyhow::Result<&PackageIndex> {
        self.index
            .get_or_try_init(|| PackageIndex::new(&self.index_url, None))
    }

    fn api_url(&self) -> anyhow::Result<Url> {
        let config = self.index()?.config()?;
        Ok(config.api)
    }
}

impl PackageSource for Registry {
    fn update(&self) -> anyhow::Result<()> {
        self.index()?.update()
    }

    fn query(&self, package_req: &PackageReq) -> anyhow::Result<Vec<Manifest>> {
        let metadata = self.index()?.get_package_metadata(package_req.name())?;
        let versions: Vec<_> = metadata
            .versions
            .iter()
            .filter(|manifest| {
                package_req.matches(&manifest.package.name, &manifest.package.version)
            })
            .cloned()
            .collect();

        Ok(versions)
    }

    fn download_package(&self, package_id: &PackageId) -> anyhow::Result<PackageContents> {
        log::info!("Downloading {}...", package_id);

        let path = format!(
            "/v1/package-contents/{}/{}/{}",
            package_id.name().scope(),
            package_id.name().name(),
            package_id.version()
        );

        let url = self.api_url()?.join(&path)?;

        let mut request = self.client.get(url).header("Wally-Version", VERSION);

        if let Some(token) = self.auth_token()? {
            request = request.header(AUTHORIZATION, format!("Bearer {}", token));
        }
        let mut response = request.send()?;

        if !response.status().is_success() {
            bail!(
                "Failed to download package {} from registry: {}\n{} {}",
                package_id,
                self.api_url()?,
                response.status(),
                response.text()?
            );
        }

        let mut data = Vec::new();
        response.read_to_end(&mut data)?;

        Ok(PackageContents::from_buffer(data))
    }

    fn fallback_sources(&self) -> anyhow::Result<Vec<PackageSourceId>> {
        let fallback_registries = self.index()?.config()?.fallback_registries;

        let sources = fallback_registries
            .into_iter()
            .map(PackageSourceId::Git)
            .collect();

        Ok(sources)
    }
}
