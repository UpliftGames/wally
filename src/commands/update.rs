use structopt::StructOpt;

/// Update all of the dependencies of this project.
#[derive(Debug, StructOpt)]
pub struct UpdateSubcommand {}

impl UpdateSubcommand {
    pub fn run(self) -> anyhow::Result<()> {
        todo!("The 'update' subcommand")
    }
}
