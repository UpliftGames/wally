mod init;
mod install;
mod login;
mod logout;
mod manifest_to_json;
mod package;
mod publish;
mod update;

pub use init::InitSubcommand;
pub use install::InstallSubcommand;
pub use login::LoginSubcommand;
pub use logout::LogoutSubcommand;
pub use manifest_to_json::ManifestToJsonSubcommand;
pub use package::PackageSubcommand;
pub use publish::PublishSubcommand;
pub use update::UpdateSubcommand;

use std::path::PathBuf;

use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(about = env!("CARGO_PKG_DESCRIPTION"))]
pub struct Args {
    #[structopt(flatten)]
    pub global: GlobalOptions,

    #[structopt(subcommand)]
    pub subcommand: Subcommand,
}

impl Args {
    pub fn run(self) -> anyhow::Result<()> {
        match self.subcommand {
            Subcommand::Publish(subcommand) => subcommand.run(self.global),
            Subcommand::Init(subcommand) => subcommand.run(),
            Subcommand::Login(subcommand) => subcommand.run(),
            Subcommand::Logout(subcommand) => subcommand.run(),
            Subcommand::Update(subcommand) => subcommand.run(),
            Subcommand::Package(subcommand) => subcommand.run(),
            Subcommand::Install(subcommand) => subcommand.run(self.global),
            Subcommand::ManifestToJson(subcommand) => subcommand.run(),
        }
    }
}

/// Options that apply to all subcommands for the CLI.
#[derive(Debug, StructOpt)]
pub struct GlobalOptions {
    /// Enable more verbose logging. Can be specified multiple times to increase
    /// verbosity further.
    #[structopt(global = true, parse(from_occurrences), long = "verbose", short)]
    pub verbosity: u8,

    /// Overrides the registry with a local registry. Usable only by tests.
    #[structopt(skip)]
    pub test_registry: Option<PathBuf>,

    /// Allows tests to specify if the package index should be temporary (to prevent multiple use conflicts). Usable only by tests.
    #[structopt(skip)]
    pub use_temp_index: bool,
}

impl Default for GlobalOptions {
    fn default() -> Self {
        Self {
            verbosity: 0,
            test_registry: None,
            use_temp_index: false,
        }
    }
}

#[derive(Debug, StructOpt)]
pub enum Subcommand {
    Init(InitSubcommand),
    Install(InstallSubcommand),
    Update(UpdateSubcommand),
    Publish(PublishSubcommand),
    Login(LoginSubcommand),
    Logout(LogoutSubcommand),
    Package(PackageSubcommand),
    ManifestToJson(ManifestToJsonSubcommand),
}
