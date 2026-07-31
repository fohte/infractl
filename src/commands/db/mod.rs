mod query;
mod targets;

use clap::Subcommand;

#[derive(Subcommand)]
pub enum DbCommands {
    /// List registered database targets
    Targets(targets::TargetsArgs),
    /// Run a read-only SQL query against a target
    Query(query::QueryArgs),
}

impl DbCommands {
    pub async fn run(&self) -> anyhow::Result<()> {
        match self {
            Self::Targets(args) => targets::run(args),
            Self::Query(args) => query::run(args).await,
        }
    }
}
