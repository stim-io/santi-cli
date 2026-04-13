use anyhow::Result;
use anyhow::anyhow;
use reqwest::StatusCode;
use uuid::Uuid;

use crate::{
    cli::{
        ChatCommand, SessionCommand, SessionCompactCommand, SessionForkCommand,
        SessionMemoryCommand, SoulCommand, SoulMemoryCommand, SoulMemorySetCommand,
    },
    config::Config,
    output,
};

mod render;
mod send;
#[cfg(test)]
mod tests;
mod types;
mod watch;

use render::{
    print_compact, print_compacts, print_effects, print_memory, print_messages, print_session,
    print_soul, print_soul_memory,
};
use send::send_message;
use types::{
    ChatOutput, ForkResponse, HealthOutput, HealthResponse, SessionCompactResponse,
    SessionCompactsResponse, SessionEffect, SessionEffectsResponse, SessionMemoryOutput,
    SessionMemoryResponse, SessionMessage, SessionMessagesResponse, SessionResponse,
    SoulMemoryResponse, SoulResponse,
};
use watch::watch_session;

pub fn health(config: &Config, json: bool) -> Result<()> {
    let client = reqwest::blocking::Client::new();
    let response = client
        .get(format!(
            "{}/api/v1/health",
            config.base_url.trim_end_matches('/')
        ))
        .send()?;
    if !response.status().is_success() {
        return Err(anyhow!("health request failed: {}", response.status()));
    }
    let health: HealthResponse = response.json()?;
    if json {
        output::json(&HealthOutput {
            status: &health.status,
            base_url: Some(&config.base_url),
        })
    } else {
        println!("status: {}", health.status);
        println!("base_url: {}", config.base_url);
        Ok(())
    }
}

pub fn chat(config: &Config, json: bool, command: ChatCommand) -> Result<()> {
    let message = output::read_message(command.message)?;
    if message.trim().is_empty() {
        return Err(anyhow!("expected message argument or stdin content"));
    }
    let client = reqwest::blocking::Client::new();
    let mut created_session_id = None;
    let session_id = match command.session {
        Some(session) => session,
        None => {
            let session = create_session(&client, config, false)?;
            created_session_id = Some(session.id.clone());
            session.id
        }
    };
    if !json && !command.raw && let Some(session_id) = created_session_id.as_deref() {
        output::stderr_line(&format!("session_id: {session_id}"))?;
    }
    send_message(
        &client,
        config,
        &session_id,
        &message,
        command.raw,
        command.wait,
        json,
    )
}

pub fn session(config: &Config, json: bool, command: SessionCommand) -> Result<()> {
    let client = reqwest::blocking::Client::new();
    match command {
        SessionCommand::Create => {
            let session = create_session(&client, config, false)?;
            if json {
                output::json(&session)
            } else {
                println!("{}", session.id);
                Ok(())
            }
        }
        SessionCommand::Get(command) => {
            let session = get_session(&client, config, &command.id)?;
            if json {
                output::json(&session)
            } else {
                print_session(&session)
            }
        }
        SessionCommand::Fork(command) => fork_session(&client, config, &command, json),
        SessionCommand::Compact(command) => compact_session(&client, config, &command, json),
        SessionCommand::Compacts(command) => {
            let compacts = list_compacts(&client, config, &command.id)?;
            if json {
                output::json(&compacts)
            } else {
                print_compacts(&compacts)
            }
        }
        SessionCommand::Send(command) => {
            let message = output::read_message(command.message)?;
            if message.trim().is_empty() {
                return Err(anyhow!(
                    "expected message argument or stdin content for session send"
                ));
            }
            send_message(
                &client,
                config,
                &command.id,
                &message,
                command.raw,
                command.wait,
                json,
            )
        }
        SessionCommand::Messages(command) => {
            let messages = list_messages(&client, config, &command.id)?;
            if json {
                output::json(&messages)
            } else {
                print_messages(&messages)
            }
        }
        SessionCommand::Effects(command) => {
            let effects = list_effects(&client, config, &command.id)?;
            if json {
                output::json(&effects)
            } else {
                print_effects(&effects)
            }
        }
        SessionCommand::Watch(command) => watch_session(&client, config, &command),
        SessionCommand::Memory { command } => match command {
            SessionMemoryCommand::Get(command) => {
                let memory = get_session_memory(&client, config, &command.id)?;
                if json {
                    output::json(&memory)
                } else {
                    print_memory(&memory)
                }
            }
            SessionMemoryCommand::Set(command) => {
                let text = output::read_message(command.text)?;
                if text.trim().is_empty() {
                    return Err(anyhow!(
                        "expected text argument or stdin content for session memory set"
                    ));
                }
                let memory = set_session_memory(&client, config, &command.id, &text)?;
                let memory = memory.into_output(&command.id);
                if json {
                    output::json(&memory)
                } else {
                    print_memory(&memory)
                }
            }
        },
    }
}

pub fn soul(config: &Config, json: bool, command: SoulCommand) -> Result<()> {
    match command {
        SoulCommand::Get => get_soul(config, json),
        SoulCommand::Memory { command } => match command {
            SoulMemoryCommand::Set(command) => set_soul_memory(config, json, command),
        },
    }
}

fn set_soul_memory(config: &Config, json: bool, command: SoulMemorySetCommand) -> Result<()> {
    let text = output::read_message(command.text)?;
    if text.trim().is_empty() {
        return Err(anyhow!(
            "expected text argument or stdin content for soul memory set"
        ));
    }

    let client = reqwest::blocking::Client::new();
    let response = client
        .put(format!(
            "{}/api/v1/soul/memory",
            config.base_url.trim_end_matches('/')
        ))
        .json(&serde_json::json!({"text": text}))
        .send()?;
    if !response.status().is_success() {
        return Err(anyhow!("soul memory set failed: {}", response.status()));
    }
    let memory: SoulMemoryResponse = response.json()?;
    if json {
        output::json(&memory)
    } else {
        print_soul_memory(&memory)
    }
}

fn create_session(
    client: &reqwest::blocking::Client,
    config: &Config,
    _json: bool,
) -> Result<SessionResponse> {
    let response = client
        .post(format!(
            "{}/api/v1/sessions",
            config.base_url.trim_end_matches('/')
        ))
        .send()?;
    if !response.status().is_success() {
        return Err(anyhow!("session create failed: {}", response.status()));
    }
    Ok(response.json()?)
}

fn get_session(client: &reqwest::blocking::Client, config: &Config, id: &str) -> Result<SessionResponse> {
    let response = client
        .get(format!(
            "{}/api/v1/sessions/{id}",
            config.base_url.trim_end_matches('/')
        ))
        .send()?;
    if !response.status().is_success() {
        return Err(anyhow!("session get failed: {}", response.status()));
    }
    Ok(response.json()?)
}

fn fork_session(
    client: &reqwest::blocking::Client,
    config: &Config,
    command: &SessionForkCommand,
    json: bool,
) -> Result<()> {
    let request_id = Uuid::new_v4().to_string();
    let response = client
        .post(format!(
            "{}/api/v1/sessions/{}/fork",
            config.base_url.trim_end_matches('/'),
            command.id
        ))
        .json(&serde_json::json!({
            "fork_point": command.fork_point,
            "request_id": request_id,
        }))
        .send()?;
    if !response.status().is_success() {
        return Err(anyhow!("session fork failed: {}", response.status()));
    }
    let fork: ForkResponse = response.json()?;
    let session = SessionResponse {
        id: fork.new_session_id,
        parent_session_id: Some(fork.parent_session_id),
        fork_point: Some(fork.fork_point),
        created_at: None,
    };
    if json {
        output::json(&session)
    } else {
        print_session(&session)
    }
}

fn get_soul(config: &Config, json: bool) -> Result<()> {
    let client = reqwest::blocking::Client::new();
    let response = client
        .get(format!(
            "{}/api/v1/soul",
            config.base_url.trim_end_matches('/')
        ))
        .send()?;
    if !response.status().is_success() {
        return Err(anyhow!("soul get failed: {}", response.status()));
    }
    let soul: SoulResponse = response.json()?;
    if json {
        output::json(&soul)
    } else {
        print_soul(&soul)
    }
}

fn list_messages(
    client: &reqwest::blocking::Client,
    config: &Config,
    id: &str,
) -> Result<Vec<SessionMessage>> {
    let response = client
        .get(format!(
            "{}/api/v1/sessions/{id}/messages",
            config.base_url.trim_end_matches('/')
        ))
        .send()?;
    if !response.status().is_success() {
        return Err(anyhow!("session messages failed: {}", response.status()));
    }
    Ok(response.json::<SessionMessagesResponse>()?.messages)
}

fn compact_session(
    client: &reqwest::blocking::Client,
    config: &Config,
    command: &SessionCompactCommand,
    json: bool,
) -> Result<()> {
    let summary = output::read_message(command.summary.clone())?;
    if summary.trim().is_empty() {
        return Err(anyhow!(
            "expected summary argument or stdin content for session compact"
        ));
    }
    let response = client
        .post(format!(
            "{}/api/v1/sessions/{}/compact",
            config.base_url.trim_end_matches('/'),
            command.id
        ))
        .json(&serde_json::json!({"summary": summary}))
        .send()?;
    if !response.status().is_success() {
        return Err(anyhow!("session compact failed: {}", response.status()));
    }
    let compact: SessionCompactResponse = response.json()?;
    if json {
        output::json(&compact)
    } else {
        print_compact(&compact)
    }
}

fn list_compacts(
    client: &reqwest::blocking::Client,
    config: &Config,
    id: &str,
) -> Result<Vec<SessionCompactResponse>> {
    let response = client
        .get(format!(
            "{}/api/v1/sessions/{id}/compacts",
            config.base_url.trim_end_matches('/')
        ))
        .send()?;
    if !response.status().is_success() {
        return Err(anyhow!("session compacts failed: {}", response.status()));
    }
    Ok(response.json::<SessionCompactsResponse>()?.compacts)
}

fn list_effects(
    client: &reqwest::blocking::Client,
    config: &Config,
    id: &str,
) -> Result<Vec<SessionEffect>> {
    let response = client
        .get(format!(
            "{}/api/v1/sessions/{id}/effects",
            config.base_url.trim_end_matches('/')
        ))
        .send()?;
    if !response.status().is_success() {
        return Err(anyhow!("session effects failed: {}", response.status()));
    }
    Ok(response.json::<SessionEffectsResponse>()?.effects)
}

fn get_session_memory(
    client: &reqwest::blocking::Client,
    config: &Config,
    id: &str,
) -> Result<SessionMemoryOutput> {
    let response = client
        .get(format!(
            "{}/api/v1/sessions/{id}/memory",
            config.base_url.trim_end_matches('/')
        ))
        .send()?;
    if response.status() == StatusCode::NOT_FOUND {
        return Ok(SessionMemoryOutput::empty(id));
    }
    if !response.status().is_success() {
        return Err(anyhow!("session memory get failed: {}", response.status()));
    }
    let memory: SessionMemoryResponse = response.json()?;
    Ok(memory.into_output(id))
}

fn set_session_memory(
    client: &reqwest::blocking::Client,
    config: &Config,
    id: &str,
    text: &str,
) -> Result<SessionMemoryResponse> {
    let response = client
        .put(format!(
            "{}/api/v1/sessions/{id}/memory",
            config.base_url.trim_end_matches('/')
        ))
        .json(&serde_json::json!({"text": text}))
        .send()?;
    if !response.status().is_success() {
        return Err(anyhow!("session memory set failed: {}", response.status()));
    }
    Ok(response.json()?)
}
