mod cli;
mod commands;
mod config;
mod registry;
mod xdg;

use clap::Parser;

use cli::{Cli, Commands};

fn main() {
    if let Err(err) = run() {
        eprintln!("Error: {err:?}");
        std::process::exit(1);
    }
}

fn run() -> anyhow::Result<()> {
    let Cli { command } = Cli::parse();
    match command {
        Commands::Db(db) => db.run(),
    }
}
