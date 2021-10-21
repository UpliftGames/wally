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

    /// Output a list of files which would be included in the package instead
    /// of creating the package
    #[structopt(short, long)]
    pub list: bool,

    /// Output file path where the package will be created
    #[structopt(long = "output", required_unless("list"))]
    pub output_path: Option<PathBuf>,
}

impl PackageSubcommand {
    pub fn run(self) -> anyhow::Result<()> {
        if self.list {
            let contents_list = PackageContents::filtered_contents(&self.project_path)?;

            for path in contents_list {
                println!("{}", path.display());
            }
        } else {
            let contents = PackageContents::pack_from_path(&self.project_path)?;
            fs_err::write(&self.output_path.unwrap(), contents.data())?;
        }

        Ok(())
    }
}
