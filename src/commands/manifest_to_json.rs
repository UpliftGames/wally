use std::path::PathBuf;

use structopt::StructOpt;

use crate::manifest::Manifest;

/// Print a Wally manifest as a line of JSON.
///
/// Used for creating the Wally package index.
#[derive(Debug, StructOpt)]
pub struct ManifestToJsonSubcommand {
    /// Path to the project to output the manifest of.
    #[structopt(long = "project-path", default_value = ".")]
    pub project_path: PathBuf,
}

impl ManifestToJsonSubcommand {
    pub fn run(self) -> anyhow::Result<()> {
        let manifest = Manifest::load(&self.project_path)?;
        println!("{}", serde_json::to_string(&manifest)?);

        Ok(())
    }
}
