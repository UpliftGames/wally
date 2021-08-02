use structopt::StructOpt;

use crate::auth::AuthStore;

/// Log into a registry.
#[derive(Debug, StructOpt)]
pub struct LoginSubcommand {
    /// Authentication token for the registry. If not specified, Wally will
    /// prompt.
    pub token: Option<String>,
}

impl LoginSubcommand {
    pub fn run(self) -> anyhow::Result<()> {
        let token = match self.token {
            Some(token) => token,
            None => {
                println!("Wally currently authenticates to registries with an API token.");
                println!("In the future, Wally will support GitHub authentication.");
                println!();
                rpassword::prompt_password_stdout("Enter token: ")?
            }
        };

        AuthStore::set_token(Some(&token))?;

        Ok(())
    }
}
