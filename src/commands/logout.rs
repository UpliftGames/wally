use structopt::StructOpt;

use crate::auth::AuthStore;

/// Log out of a registry.
#[derive(Debug, StructOpt)]
pub struct LogoutSubcommand {}

impl LogoutSubcommand {
    pub fn run(self) -> anyhow::Result<()> {
        AuthStore::set_token(None)?;

        Ok(())
    }
}
