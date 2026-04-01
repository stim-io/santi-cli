mod backend;
mod cli;
mod config;
mod output;

use anyhow::Result;
use clap::Parser;
use cli::{Backend, Cli, Command};
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

    let config = config::resolve(cli.backend, cli.base_url)?;
    debug!(backend = ?config.backend, base_url = %config.base_url, "starting santi-cli");

    match cli.command {
        Command::Health => match config.backend {
            Backend::Http => backend::http::health(&config, cli.json),
            Backend::Local => backend::local::health(),
        },
        Command::Chat(command) => match config.backend {
            Backend::Http => backend::http::chat(&config, cli.json, command),
            Backend::Local => backend::local::chat(),
        },
        Command::Soul { command } => match config.backend {
            Backend::Http => backend::http::soul(&config, cli.json, command),
            Backend::Local => match command {
                cli::SoulCommand::Get => backend::local::soul(),
                cli::SoulCommand::Memory { command } => match command {
                    cli::SoulMemoryCommand::Set => backend::local::soul_memory_set(),
                },
            },
        },
        Command::Session { command } => match config.backend {
            Backend::Http => backend::http::session(&config, cli.json, command),
            Backend::Local => backend::local::session(),
        },
    }
}

fn init_tracing(level: &str) -> Result<()> {
    let filter =
        EnvFilter::try_new(level).or_else(|_| EnvFilter::try_new(format!("santi_cli={level}")))?;
    tracing_subscriber::fmt().with_env_filter(filter).init();
    Ok(())
}
