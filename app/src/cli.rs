use clap::{Parser, Subcommand, ValueEnum};
use serde::Deserialize;

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Backend {
    Http,
    Local,
}

#[derive(Debug, Parser)]
#[command(name = "santi-cli")]
pub struct Cli {
    #[arg(long, value_enum)]
    pub backend: Option<Backend>,

    #[arg(long)]
    pub base_url: Option<String>,

    #[arg(long, default_value = "info")]
    pub log_level: String,

    #[arg(long, global = true)]
    pub json: bool,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    Health,
    Chat(ChatCommand),
    Soul {
        #[command(subcommand)]
        command: SoulCommand,
    },
    Session {
        #[command(subcommand)]
        command: SessionCommand,
    },
}

#[derive(Debug, clap::Args)]
pub struct ChatCommand {
    #[arg(long)]
    pub session: Option<String>,

    #[arg(long)]
    pub raw: bool,

    #[arg(long)]
    pub wait: bool,

    pub message: Option<String>,
}

#[derive(Debug, Subcommand)]
pub enum SessionCommand {
    Create,
    Get(SessionIdCommand),
    Fork(SessionForkCommand),
    Compact(SessionCompactCommand),
    Compacts(SessionIdCommand),
    Send(SessionSendCommand),
    Messages(SessionIdCommand),
    Effects(SessionIdCommand),
    Memory {
        #[command(subcommand)]
        command: SessionMemoryCommand,
    },
}

#[derive(Debug, Subcommand)]
pub enum SessionMemoryCommand {
    Get(SessionIdCommand),
    Set(SessionIdCommand),
}

#[derive(Debug, Subcommand)]
pub enum SoulCommand {
    Get,
    Memory {
        #[command(subcommand)]
        command: SoulMemoryCommand,
    },
}

#[derive(Debug, Subcommand)]
pub enum SoulMemoryCommand {
    Set,
}

#[derive(Debug, clap::Args)]
pub struct SessionIdCommand {
    pub id: String,
}

#[derive(Debug, clap::Args)]
pub struct SessionSendCommand {
    pub id: String,

    #[arg(long)]
    pub raw: bool,

    #[arg(long)]
    pub wait: bool,
}

#[derive(Debug, clap::Args)]
pub struct SessionCompactCommand {
    pub id: String,
}

#[derive(Debug, clap::Args)]
pub struct SessionForkCommand {
    pub id: String,

    #[arg(long, value_name = "n")]
    pub fork_point: i64,
}
