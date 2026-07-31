mod targets;

use clap::Subcommand;

#[derive(Subcommand)]
pub enum DbCommands {
    /// List registered database targets
    Targets(targets::TargetsArgs),
}

impl DbCommands {
    pub fn run(&self) -> anyhow::Result<()> {
        match self {
            Self::Targets(args) => targets::run(args),
        }
    }
}
