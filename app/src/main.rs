mod backend;
mod cli;
mod config;
mod output;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Command};
use tracing::debug;
use tracing_subscriber::EnvFilter;

fn main() {
    if let Err(err) = run() {
        eprintln!("error: {err}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    init_tracing(&cli.log_level)?;

    let config = config::resolve(cli.base_url)?;
    debug!(base_url = %config.base_url, "starting santi-cli");

    match cli.command {
        Command::Health => backend::http::health(&config, cli.json),
        Command::Chat(command) => backend::http::chat(&config, cli.json, command),
        Command::Soul { command } => backend::http::soul(&config, cli.json, command),
        Command::Session { command } => backend::http::session(&config, cli.json, command),
    }
}

fn init_tracing(level: &str) -> Result<()> {
    let filter =
        EnvFilter::try_new(level).or_else(|_| EnvFilter::try_new(format!("santi_cli={level}")))?;
    tracing_subscriber::fmt().with_env_filter(filter).init();
    Ok(())
}
