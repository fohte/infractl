mod cli;
mod commands;
mod config;
mod registry;
mod xdg;

use clap::Parser;

use cli::{Cli, Commands};

#[tokio::main(flavor = "current_thread")]
async fn main() {
    if let Err(err) = run().await {
        eprintln!("Error: {err:?}");
        std::process::exit(1);
    }
}

async fn run() -> anyhow::Result<()> {
    let Cli { command } = Cli::parse();

    // Avoid running the updater twice when `infractl update` was requested.
    if !matches!(command, Commands::Update) {
        commands::update::auto_update().await;
    }

    match command {
        Commands::Db(db) => db.run().await,
        Commands::Update => commands::update::run().await,
    }
}
