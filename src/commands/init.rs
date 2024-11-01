use std::env::current_dir;
use std::path::PathBuf;

use anyhow::{bail, Context};
use structopt::StructOpt;
use toml_edit::{value, Document};

use crate::manifest::MANIFEST_FILE_NAME;

const DEFAULT_MANIFEST: &str = r#"[package]
name = "placeholder/placeholder"
version = "0.1.0"
registry = "https://github.com/UpliftGames/wally-index"
realm = "shared"

[dependencies]
"#;

/// Initialize a new Wally project.
#[derive(Debug, StructOpt)]
pub struct InitSubcommand {
    /// The path to the project to initialize. Defaults to the current
    /// directory.
    path: Option<PathBuf>,
}

impl InitSubcommand {
    pub fn run(self) -> anyhow::Result<()> {
        let path = match self.path {
            Some(path) => path,
            None => current_dir()?,
        };

        let manifest_path = path.join(MANIFEST_FILE_NAME);

        match fs_err::metadata(&manifest_path) {
            Ok(_) => bail!(
                "There is already a Wally project in this directory. Manifest file ({}) already \
                 exists.",
                MANIFEST_FILE_NAME
            ),

            Err(err) => {
                if err.kind() == std::io::ErrorKind::NotFound {
                    // Perfect! This is the state that we want
                } else {
                    bail!(
                        "Error accessing manifest file ({}): {}",
                        MANIFEST_FILE_NAME,
                        err
                    );
                }
            }
        }

        let canonical = fs_err::canonicalize(&path);
        let package_name = match &canonical {
            Ok(canonical) => canonical
                .file_name()
                .and_then(|name| name.to_str())
                .context("Folder name contained invalid Unicode")?,
            Err(_) => "unknown",
        };

        let mut doc = DEFAULT_MANIFEST
            .parse::<Document>()
            .expect("Built-in default manifest was invalid TOML");

        let full_name = format!("{}/{}", whoami::username(), package_name)
            .to_lowercase()
            .replace(" ", "-");
        doc["package"]["name"] = value(full_name.clone());

        fs_err::write(manifest_path, doc.to_string())?;
        println!("Initialized project {} in {}", full_name, path.display());

        Ok(())
    }
}
