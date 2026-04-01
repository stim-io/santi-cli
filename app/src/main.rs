use std::io::{self, Write};

use anyhow::Result;
use clap::{Parser, ValueEnum};
use serde::Serialize;
use tracing::debug;
use tracing_subscriber::EnvFilter;

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
enum Backend {
    Http,
    Local,
}

#[derive(Debug, Parser)]
#[command(name = "santi-cli")]
#[command(about = "Standalone CLI scaffold for santi")]
struct Cli {
    #[arg(long, value_enum, env = "SANTI_CLI_BACKEND", default_value = "http")]
    backend: Backend,

    #[arg(long, env = "SANTI_CLI_LOG_LEVEL", default_value = "info")]
    log_level: String,

    #[arg(long, global = true)]
    json: bool,

    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Debug, clap::Subcommand)]
enum Command {
    /// Minimal scaffold health check
    Health,
}

#[derive(Debug, Serialize)]
struct HealthOutput<'a> {
    status: &'a str,
    backend: &'a str,
    mode: &'a str,
}

fn main() {
    if let Err(err) = run() {
        let _ = writeln!(io::stderr(), "error: {err}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    init_tracing(&cli.log_level)?;

    debug!(backend = ?cli.backend, json = cli.json, "starting santi-cli scaffold");

    match cli.command.unwrap_or(Command::Health) {
        Command::Health => print_health(cli.backend, cli.json)?,
    }

    Ok(())
}

fn init_tracing(level: &str) -> Result<()> {
    let filter =
        EnvFilter::try_new(level).or_else(|_| EnvFilter::try_new(format!("santi_cli={level}")))?;

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(io::stderr)
        .without_time()
        .init();

    Ok(())
}

fn print_health(backend: Backend, json: bool) -> Result<()> {
    let backend_name = match backend {
        Backend::Http => "http",
        Backend::Local => "local",
    };

    let output = HealthOutput {
        status: "ok",
        backend: backend_name,
        mode: "scaffold",
    };

    if json {
        serde_json::to_writer_pretty(io::stdout(), &output)?;
        writeln!(io::stdout())?;
    } else {
        println!("status: ok");
        println!("backend: {backend_name}");
        println!("mode: scaffold");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::Backend;

    #[test]
    fn backend_names_are_stable() {
        assert_eq!(format!("{:?}", Backend::Http), "Http");
        assert_eq!(format!("{:?}", Backend::Local), "Local");
    }
}
