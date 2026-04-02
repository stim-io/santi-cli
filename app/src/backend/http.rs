use std::{io::Read, thread, time::Duration};

use anyhow::{anyhow, Result};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    cli::{
        ChatCommand, SessionCommand, SessionCompactCommand, SessionForkCommand,
        SessionMemoryCommand, SoulCommand, SoulMemoryCommand,
    },
    config::Config,
    output,
};

#[derive(Debug, Deserialize)]
struct HealthResponse {
    status: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct SessionResponse {
    id: String,
    parent_session_id: Option<String>,
    fork_point: Option<i64>,
    created_at: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ForkResponse {
    new_session_id: String,
    parent_session_id: String,
    fork_point: i64,
}

#[derive(Debug, Deserialize, Serialize)]
struct SessionMessage {
    id: String,
    actor_type: String,
    actor_id: String,
    session_seq: i64,
    content_text: String,
    state: String,
    created_at: String,
}

#[derive(Debug, Serialize)]
struct HealthOutput<'a> {
    status: &'a str,
    base_url: Option<&'a str>,
}

#[derive(Debug, Serialize)]
struct ChatOutput<'a> {
    session_id: &'a str,
    output_text: &'a str,
}

#[derive(Debug, Deserialize)]
struct SessionMessagesResponse {
    messages: Vec<SessionMessage>,
}

#[derive(Debug, Deserialize, Serialize)]
struct SessionCompactResponse {
    id: String,
    turn_id: String,
    summary: String,
    start_session_seq: i64,
    end_session_seq: i64,
    created_at: String,
}

#[derive(Debug, Deserialize)]
struct SessionCompactsResponse {
    compacts: Vec<SessionCompactResponse>,
}

#[derive(Debug, Deserialize, Serialize)]
struct SessionEffect {
    id: String,
    name: String,
    value: serde_json::Value,
    state: String,
    created_at: Option<String>,
    updated_at: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
struct SoulMemoryResponse {
    id: String,
    memory: String,
    updated_at: String,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum SseEvent {
    #[serde(rename = "response.output_text.delta")]
    Delta { delta: String },
    #[serde(rename = "response.completed")]
    Completed,
}

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
    let session_id = match command.session {
        Some(session) => session,
        None => create_session(&client, config, false)?.id,
    };
    eprintln!("session: {session_id}");
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
            let message = output::read_message(None)?;
            if message.trim().is_empty() {
                return Err(anyhow!("expected stdin content for session send"));
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
                let text = output::read_message(None)?;
                if text.trim().is_empty() {
                    return Err(anyhow!("expected stdin content for session memory set"));
                }
                let memory = set_session_memory(&client, config, &command.id, &text)?;
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
            SoulMemoryCommand::Set => set_soul_memory(config, json),
        },
    }
}

fn set_soul_memory(config: &Config, json: bool) -> Result<()> {
    let text = output::read_message(None)?;
    if text.trim().is_empty() {
        return Err(anyhow!("expected stdin content for soul memory set"));
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
fn get_session(
    client: &reqwest::blocking::Client,
    config: &Config,
    id: &str,
) -> Result<SessionResponse> {
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

#[derive(Debug, Deserialize, Serialize)]
struct SoulResponse {
    id: String,
    memory: String,
    created_at: String,
    updated_at: String,
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
    let summary = output::read_message(None)?;
    if summary.trim().is_empty() {
        return Err(anyhow!("expected stdin content for session compact"));
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

fn print_session(session: &SessionResponse) -> Result<()> {
    println!("id: {}", session.id);
    if let Some(parent_session_id) = &session.parent_session_id {
        println!("parent_session_id: {parent_session_id}");
    }
    if let Some(fork_point) = session.fork_point {
        println!("fork_point: {fork_point}");
    }
    if let Some(created_at) = &session.created_at {
        println!("created_at: {created_at}");
    }
    Ok(())
}

fn print_compact(compact: &SessionCompactResponse) -> Result<()> {
    println!("id: {}", compact.id);
    println!("turn_id: {}", compact.turn_id);
    println!("summary: {}", compact.summary);
    println!("start_session_seq: {}", compact.start_session_seq);
    println!("end_session_seq: {}", compact.end_session_seq);
    println!("created_at: {}", compact.created_at);
    Ok(())
}

fn print_compacts(compacts: &[SessionCompactResponse]) -> Result<()> {
    if compacts.is_empty() {
        println!("no session compacts recorded");
        return Ok(());
    }
    println!("compacts: {}", compacts.len());
    for compact in compacts {
        println!(
            "- {} [{}..{}] {}",
            compact.id, compact.start_session_seq, compact.end_session_seq, compact.summary
        );
    }
    Ok(())
}

fn print_messages(messages: &[SessionMessage]) -> Result<()> {
    if messages.is_empty() {
        println!("no session messages recorded");
        return Ok(());
    }
    println!("messages: {}", messages.len());
    for message in messages {
        println!(
            "- {} [{}] {}",
            message.actor_type, message.session_seq, message.content_text
        );
    }
    Ok(())
}

fn print_memory(memory: &SessionMemoryResponse) -> Result<()> {
    println!("id: {}", memory.id);
    println!("memory: {}", memory.memory);
    println!("updated_at: {}", memory.updated_at);
    Ok(())
}

fn print_soul(soul: &SoulResponse) -> Result<()> {
    println!("id: {}", soul.id);
    println!("memory: {}", soul.memory);
    println!("created_at: {}", soul.created_at);
    println!("updated_at: {}", soul.updated_at);
    Ok(())
}

fn print_soul_memory(memory: &SoulMemoryResponse) -> Result<()> {
    println!("id: {}", memory.id);
    println!("memory: {}", memory.memory);
    println!("updated_at: {}", memory.updated_at);
    Ok(())
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

#[derive(Debug, Deserialize)]
struct SessionEffectsResponse {
    effects: Vec<SessionEffect>,
}

fn print_effects(effects: &[SessionEffect]) -> Result<()> {
    if effects.is_empty() {
        println!("no session effects recorded");
        return Ok(());
    }

    println!("effects: {}", effects.len());
    for effect in effects {
        let value = match &effect.value {
            serde_json::Value::String(text) => text.clone(),
            other => other.to_string(),
        };
        println!("- {} [{}] {}", effect.name, effect.state, value);
    }
    Ok(())
}

#[derive(Debug, Deserialize, Serialize)]
struct SessionMemoryResponse {
    id: String,
    memory: String,
    updated_at: String,
}

fn get_session_memory(
    client: &reqwest::blocking::Client,
    config: &Config,
    id: &str,
) -> Result<SessionMemoryResponse> {
    let response = client
        .get(format!(
            "{}/api/v1/sessions/{id}/memory",
            config.base_url.trim_end_matches('/')
        ))
        .send()?;
    if !response.status().is_success() {
        return Err(anyhow!("session memory get failed: {}", response.status()));
    }
    Ok(response.json()?)
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
fn send_message(
    client: &reqwest::blocking::Client,
    config: &Config,
    session_id: &str,
    message: &str,
    raw: bool,
    wait: bool,
    json: bool,
) -> Result<()> {
    let url = format!(
        "{}/api/v1/sessions/{session_id}/send",
        config.base_url.trim_end_matches('/')
    );
    let body = serde_json::json!({"content":[{"type":"text","text":message}]});
    let mut response = loop {
        let response = client.post(&url).json(&body).send()?;
        if response.status() != StatusCode::CONFLICT || !wait {
            break response;
        }
        thread::sleep(Duration::from_millis(350));
    };
    if !response.status().is_success() {
        return Err(anyhow!("send request failed: {}", response.status()));
    }
    let mut text = String::new();
    let mut output_text = String::new();
    response.read_to_string(&mut text)?;
    for line in text.lines() {
        if let Some(payload) = line.strip_prefix("data: ") {
            if payload.is_empty() || payload == "[DONE]" {
                continue;
            }
            if let Ok(event) = serde_json::from_str::<SseEvent>(payload) {
                match event {
                    SseEvent::Delta { delta } => {
                        output_text.push_str(&delta);
                        if raw {
                            println!(
                                "{{\"type\":\"response.output_text.delta\",\"delta\":{}}}",
                                serde_json::to_string(&delta)?
                            );
                        } else if !json {
                            output::stream_text(&delta)?;
                        }
                    }
                    SseEvent::Completed => {
                        if raw {
                            println!("{{\"type\":\"response.completed\"}}");
                        }
                    }
                }
            }
        }
    }
    if !raw {
        if json {
            output::json(&ChatOutput {
                session_id,
                output_text: &output_text,
            })?;
        } else {
            println!();
        }
    }
    Ok(())
}
