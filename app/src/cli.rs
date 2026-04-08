use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(
    name = "santi-cli",
    about = "HTTP CLI for health, chat, soul, and session workflows",
    long_about = None
)]
pub struct Cli {
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
    #[command(about = "Check runtime health")]
    Health,
    #[command(about = "Send a chat message; reads stdin when message is omitted")]
    Chat(ChatCommand),
    #[command(about = "Inspect and update soul state")]
    Soul {
        #[command(subcommand)]
        command: SoulCommand,
    },
    #[command(about = "Create, inspect, and mutate sessions")]
    Session {
        #[command(subcommand)]
        command: SessionCommand,
    },
}

#[derive(Debug, clap::Args)]
#[command(about = "Send one message to a session or a new auto-created session")]
pub struct ChatCommand {
    #[arg(long)]
    pub session: Option<String>,

    #[arg(long)]
    pub raw: bool,

    #[arg(long)]
    pub wait: bool,

    #[arg(help = "Message text; if omitted, reads all stdin")]
    pub message: Option<String>,
}

#[derive(Debug, Subcommand)]
pub enum SessionCommand {
    #[command(about = "Create a new session")]
    Create,
    #[command(about = "Show one session")]
    Get(SessionIdCommand),
    #[command(about = "Fork a session from a prior turn")]
    Fork(SessionForkCommand),
    #[command(about = "Create a compact from text or stdin summary")]
    Compact(SessionCompactCommand),
    #[command(about = "List session compacts")]
    Compacts(SessionIdCommand),
    #[command(about = "Send text or stdin content to a session")]
    Send(SessionSendCommand),
    #[command(about = "List session messages")]
    Messages(SessionIdCommand),
    #[command(about = "List session effects")]
    Effects(SessionIdCommand),
    #[command(about = "Inspect and replace session memory")]
    Memory {
        #[command(subcommand)]
        command: SessionMemoryCommand,
    },
}

#[derive(Debug, Subcommand)]
pub enum SessionMemoryCommand {
    #[command(about = "Show session memory")]
    Get(SessionIdCommand),
    #[command(about = "Replace session memory from text or stdin")]
    Set(SessionMemorySetCommand),
}

#[derive(Debug, Subcommand)]
pub enum SoulCommand {
    #[command(about = "Show soul state")]
    Get,
    #[command(about = "Replace soul memory from text or stdin")]
    Memory {
        #[command(subcommand)]
        command: SoulMemoryCommand,
    },
}

#[derive(Debug, Subcommand)]
pub enum SoulMemoryCommand {
    #[command(about = "Replace soul memory from text or stdin")]
    Set(SoulMemorySetCommand),
}

#[derive(Debug, clap::Args)]
#[command(about = "Target a single session id")]
pub struct SessionIdCommand {
    pub id: String,
}

#[derive(Debug, clap::Args)]
#[command(about = "Send text or stdin content to an existing session")]
pub struct SessionSendCommand {
    pub id: String,

    #[arg(help = "Message text; if omitted, reads all stdin")]
    pub message: Option<String>,

    #[arg(long)]
    pub raw: bool,

    #[arg(long)]
    pub wait: bool,
}

#[derive(Debug, clap::Args)]
#[command(about = "Create a compact using summary text or stdin")]
pub struct SessionCompactCommand {
    pub id: String,

    #[arg(help = "Summary text; if omitted, reads all stdin")]
    pub summary: Option<String>,
}

#[derive(Debug, clap::Args)]
#[command(about = "Replace session memory using text or stdin")]
pub struct SessionMemorySetCommand {
    pub id: String,

    #[arg(help = "Memory text; if omitted, reads all stdin")]
    pub text: Option<String>,
}

#[derive(Debug, clap::Args)]
#[command(about = "Replace soul memory using text or stdin")]
pub struct SoulMemorySetCommand {
    #[arg(help = "Memory text; if omitted, reads all stdin")]
    pub text: Option<String>,
}

#[derive(Debug, clap::Args)]
#[command(about = "Fork a session from a specific session sequence")]
pub struct SessionForkCommand {
    pub id: String,

    #[arg(long, value_name = "n")]
    pub fork_point: i64,
}

#[cfg(test)]
mod tests {
    use super::{
        Cli, Command, SessionCommand, SessionMemoryCommand, SoulCommand, SoulMemoryCommand,
    };
    use clap::Parser;

    #[test]
    fn session_send_accepts_optional_message() {
        let cli =
            Cli::try_parse_from(["santi-cli", "session", "send", "session-123", "hello"]).unwrap();

        match cli.command {
            Command::Session {
                command: SessionCommand::Send(command),
            } => {
                assert_eq!(command.id, "session-123");
                assert_eq!(command.message.as_deref(), Some("hello"));
            }
            other => panic!("unexpected command: {other:?}"),
        }
    }

    #[test]
    fn session_memory_set_accepts_optional_text() {
        let cli = Cli::try_parse_from([
            "santi-cli",
            "session",
            "memory",
            "set",
            "session-123",
            "remember this",
        ])
        .unwrap();

        match cli.command {
            Command::Session {
                command:
                    SessionCommand::Memory {
                        command: SessionMemoryCommand::Set(command),
                    },
            } => {
                assert_eq!(command.id, "session-123");
                assert_eq!(command.text.as_deref(), Some("remember this"));
            }
            other => panic!("unexpected command: {other:?}"),
        }
    }

    #[test]
    fn soul_memory_set_accepts_optional_text() {
        let cli =
            Cli::try_parse_from(["santi-cli", "soul", "memory", "set", "core memory"]).unwrap();

        match cli.command {
            Command::Soul {
                command:
                    SoulCommand::Memory {
                        command: SoulMemoryCommand::Set(command),
                    },
            } => {
                assert_eq!(command.text.as_deref(), Some("core memory"));
            }
            other => panic!("unexpected command: {other:?}"),
        }
    }
}
