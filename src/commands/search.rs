use std::path::PathBuf;

use anyhow::bail;
use crossterm::style::Color;
use crossterm::style::SetForegroundColor;
use crossterm::style::Stylize;
use reqwest::{blocking::Client, header::AUTHORIZATION};
use serde::Deserialize;
use structopt::StructOpt;

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
        let auth_store = AuthStore::load()?;

        let mut package_indexes: Vec<PackageIndex> = Vec::new();
        let mut registry_order = vec![manifest.package.registry];
        let mut registry_index = 0;

        while registry_index < registry_order.len() {
            let registry = &registry_order[registry_index];
            let url = url::Url::parse(&registry)?;
            let package_index = PackageIndex::new(&url, None)?;
            let fallback_registries = package_index.config()?.fallback_registries;

            package_indexes.push(package_index);

            for fallback in fallback_registries {
                // Prevent circular references by only adding new registries
                if !registry_order.contains(&&fallback) {
                    registry_order.push(fallback);
                }
            }

            registry_index += 1;
        }

        let client = Client::new();
        let mut results: Vec<SearchResult> = Vec::new();

        for package_index in package_indexes {
            log::info!("Searching package index {}...", package_index.url());

            let api = package_index.config()?.api;
            let auth = auth_store.tokens.get(api.as_str());

            let mut request = client
                .get(api.join("/v1/package-search/")?)
                .query(&[("query", &self.query)]);

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

            let package_results: Vec<SearchResult> = response.json()?;
            results.extend(package_results);
        }

        println!();

        for result in &mut results {
            print!("{}{}/", SetForegroundColor(Color::DarkGrey), result.scope);
            print!("{}{}", SetForegroundColor(Color::Reset), result.name);
            print!(
                "{}@{}{}",
                SetForegroundColor(Color::DarkGrey),
                SetForegroundColor(Color::Green),
                result.versions.pop().unwrap(),
            );

            if !result.versions.is_empty() {
                print!(
                    "{} ({})",
                    SetForegroundColor(Color::DarkGrey),
                    result.versions.join(", ")
                );
            }

            println!("{}", SetForegroundColor(Color::Reset));

            if let Some(description) = &result.description {
                println!("    {}", description);
                println!();
            }
        }

        println!();

        Ok(())
    }
}

#[derive(Deserialize)]
struct SearchResult {
    pub scope: String,
    pub name: String,
    pub versions: Vec<String>,
    pub description: Option<String>,
}
