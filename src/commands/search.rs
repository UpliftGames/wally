use std::{fmt, path::PathBuf};

use anyhow::{bail, Context};
use reqwest::{blocking::Client, header::AUTHORIZATION};
use serde::Deserialize;
use structopt::StructOpt;
use termion::color::{self, Color};

use crate::{auth::AuthStore, manifest::Manifest, package_index::PackageIndex};

/// Search a registry for packages matching a query.
#[derive(Debug, StructOpt)]
pub struct SearchSubcommand {
    /// Path to a project to decide how to search
    #[structopt(long = "project-path", default_value = ".")]
    pub project_path: PathBuf,

    /// The query to be dispatched to the search endpoint
    pub query: String,
}

impl SearchSubcommand {
    pub fn run(self) -> anyhow::Result<()> {
        let manifest = Manifest::load(&self.project_path)?;
        let registry = url::Url::parse(&manifest.package.registry)?;
        let auth_store = AuthStore::load()?;
        let package_index = PackageIndex::new(&registry, None)?;
        let api = package_index.config()?.api;

        let auth = auth_store.tokens.get(api.as_str());

        let client = Client::new();
        let mut request = client.get(api.join(&format!("/v1/package-search/{}", self.query))?);

        if let Some(auth) = auth {
            request = request.header(AUTHORIZATION, format!("Bearer {}", auth));
        }

        let response = request.send()?;

        if !response.status().is_success() {
            bail!(
                "Failed to search: {} {}",
                response.status(),
                response.text()?
            );
        }

        let results: Vec<String> = response.json()?;
        let results: Vec<SearchResult> = results
            .iter()
            .map(|s| serde_json::from_str(s).unwrap())
            .collect();

        for result in &results {
            print!("{}{}/", color::Fg(color::LightBlack), result.scope[0]);
            print!("{}{}", color::Fg(color::Reset), result.name[0]);
            println!(
                "{}@{}{}",
                color::Fg(color::LightBlack),
                color::Fg(color::Green),
                result.version[0]
            );
            print!("{}", color::Fg(color::Reset));

            if let Some(description) = &result.description {
                println!("    {}", description[0]);
                println!();
            }
        }

        Ok(())
    }
}

#[derive(Deserialize)]
struct SearchResult {
    pub scope: Vec<String>,
    pub name: Vec<String>,
    pub version: Vec<String>,
    pub description: Option<Vec<String>>,
}
impl fmt::Debug for SearchResult {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}/{} {} {:?}",
            self.scope[0], self.name[0], self.version[0], self.description
        )
    }
}
