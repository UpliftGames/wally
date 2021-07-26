use std::path::PathBuf;

use structopt::StructOpt;

use crate::package_contents::PackageContents;

/// Package the project as a tarball suitable for uploading to a package
/// registry.
#[derive(Debug, StructOpt)]
pub struct PackageSubcommand {
    /// Path to the project to turn into a package ready for upload to an index
    #[structopt(long = "project-path", default_value = ".")]
    pub project_path: PathBuf,

    /// Output file path where the package will be created
    #[structopt(long = "output")]
    pub output_path: PathBuf,
}

impl PackageSubcommand {
    pub fn run(self) -> anyhow::Result<()> {
        let contents = PackageContents::pack_from_path(&self.project_path)?;
        fs_err::write(&self.output_path, contents.data())?;
        Ok(())
    }
}
