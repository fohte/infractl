use clap::{Parser, Subcommand};

use crate::commands::db::DbCommands;

#[derive(Parser)]
#[command(
    version,
    about,
    subcommand_required = true,
    arg_required_else_help = true
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Query CloudNativePG-managed Postgres clusters
    #[command(subcommand)]
    Db(DbCommands),
    /// Update infractl to the latest release
    Update,
}
