use std::{
    collections::BTreeSet,
    io::{BufRead, BufReader, Read},
    sync::mpsc,
    thread,
    time::Duration,
};

use anyhow::Result;
use anyhow::anyhow;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    cli::{
        ChatCommand, SessionCommand, SessionCompactCommand, SessionForkCommand,
        SessionMemoryCommand, SessionWatchCommand, SoulCommand, SoulMemoryCommand,
        SoulMemorySetCommand,
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

#[derive(Debug, Clone, Deserialize, Serialize)]
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

#[derive(Debug, Serialize)]
struct SessionMemoryOutput {
    session_id: String,
    memory: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    memory_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    updated_at: Option<String>,
    exists: bool,
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

#[derive(Debug, Clone, Deserialize, Serialize)]
struct SessionEffect {
    id: String,
    session_id: String,
    effect_type: String,
    idempotency_key: String,
    status: String,
    source_hook_id: String,
    source_turn_id: String,
    result_ref: Option<String>,
    error_text: Option<String>,
    created_at: String,
    updated_at: String,
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

#[derive(Debug, Deserialize)]
struct SseErrorEnvelope {
    error: SseErrorBody,
}

#[derive(Debug, Deserialize)]
struct SseErrorBody {
    code: String,
    message: String,
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
    let mut created_session_id = None;
    let session_id = match command.session {
        Some(session) => session,
        None => {
            let session = create_session(&client, config, false)?;
            created_session_id = Some(session.id.clone());
            session.id
        }
    };
    if !json
        && !command.raw
        && let Some(session_id) = created_session_id.as_deref()
    {
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

fn watch_session(
    client: &reqwest::blocking::Client,
    config: &Config,
    command: &SessionWatchCommand,
) -> Result<()> {
    let mut snapshot = SessionWatchSnapshot::load(client, config, &command.id)?;
    println!(
        ":: watch session_id={} status=started transport=sse idle_ms={} baseline_messages={} baseline_effects={} last_seq={}",
        snapshot.session_id,
        command.idle_ms,
        snapshot.messages.len(),
        snapshot.effects.len(),
        snapshot.latest_seq
    );

    let response = client
        .get(format!(
            "{}/api/v1/sessions/{}/watch",
            config.base_url.trim_end_matches('/'),
            command.id
        ))
        .send()?;
    if !response.status().is_success() {
        return Err(anyhow!("session watch failed: {}", response.status()));
    }

    let (tx, rx) = mpsc::channel::<Result<Option<String>, String>>();
    thread::spawn(move || {
        let reader = BufReader::new(response);
        for line in reader.lines() {
            match line {
                Ok(line) => {
                    if tx.send(Ok(Some(line))).is_err() {
                        return;
                    }
                }
                Err(err) => {
                    let _ = tx.send(Err(err.to_string()));
                    return;
                }
            }
        }
        let _ = tx.send(Ok(None));
    });

    loop {
        let Some(line) = receive_watch_line(&rx, command.idle_ms, &command.id)? else {
            return Ok(());
        };

        let Some(event) = parse_watch_sse_line(&line)? else {
            continue;
        };

        let _ = handle_watch_event(client, config, event, &mut snapshot)?;
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
            "- id: {} turn_id: {} seq: [{}..{}] {}",
            compact.id,
            compact.turn_id,
            compact.start_session_seq,
            compact.end_session_seq,
            preview_text(&compact.summary)
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
            "- id: {} seq: {} actor: {}:{} state: {} {}",
            message.id,
            message.session_seq,
            message.actor_type,
            message.actor_id,
            message.state,
            preview_text(&message.content_text)
        );
    }
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

#[derive(Debug, Deserialize)]
struct SessionWatchSnapshotResponse {
    session_id: String,
    latest_seq: i64,
    messages: Vec<SessionMessage>,
    effects: Vec<SessionEffect>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum WatchEvent {
    Connected {
        session_id: String,
        latest_seq: i64,
    },
    StateChanged {
        session_id: String,
        state: String,
    },
    MessageChanged {
        session_id: String,
        message_id: String,
        session_seq: i64,
        change: String,
        actor_type: String,
    },
    ActivityChanged {
        session_id: String,
        activity: String,
        state: String,
        label: Option<String>,
    },
}

#[derive(Debug, Clone)]
struct SessionWatchSnapshot {
    session_id: String,
    latest_seq: i64,
    messages: Vec<SessionMessage>,
    effects: Vec<SessionEffect>,
}

impl SessionWatchSnapshot {
    fn load(client: &reqwest::blocking::Client, config: &Config, session_id: &str) -> Result<Self> {
        let response = client
            .get(format!(
                "{}/api/v1/sessions/{session_id}/watch-snapshot",
                config.base_url.trim_end_matches('/'),
            ))
            .send()?;
        if !response.status().is_success() {
            return Err(anyhow!(
                "session watch snapshot failed: {}",
                response.status()
            ));
        }
        let snapshot: SessionWatchSnapshotResponse = response.json()?;
        Ok(Self {
            session_id: snapshot.session_id,
            latest_seq: snapshot.latest_seq,
            messages: snapshot.messages,
            effects: snapshot.effects,
        })
    }

    fn new_messages_since<'a>(&'a self, previous: &'a Self) -> Vec<&'a SessionMessage> {
        let seen = previous
            .messages
            .iter()
            .map(|message| message.id.as_str())
            .collect::<BTreeSet<_>>();

        self.messages
            .iter()
            .filter(|message| !seen.contains(message.id.as_str()))
            .collect()
    }

    fn new_effects_since<'a>(&'a self, previous: &'a Self) -> Vec<&'a SessionEffect> {
        let seen = previous
            .effects
            .iter()
            .map(|effect| effect.id.as_str())
            .collect::<BTreeSet<_>>();

        self.effects
            .iter()
            .filter(|effect| !seen.contains(effect.id.as_str()))
            .collect()
    }
}

fn handle_watch_event(
    client: &reqwest::blocking::Client,
    config: &Config,
    event: WatchEvent,
    snapshot: &mut SessionWatchSnapshot,
) -> Result<bool> {
    match event {
        WatchEvent::Connected {
            session_id,
            latest_seq,
        } => {
            println!(
                ":: watch session_id={} status=connected last_seq={}",
                session_id, latest_seq
            );
            Ok(false)
        }
        WatchEvent::StateChanged { session_id, state } => {
            println!(":: state session_id={} state={}", session_id, state);
            Ok(true)
        }
        WatchEvent::ActivityChanged {
            session_id,
            activity,
            state,
            label,
        } => {
            let mut line = format!(
                ":: activity session_id={} activity={} state={}",
                session_id, activity, state
            );
            if let Some(label) = label {
                line.push_str(&format!(" label={}", preview_text(&label)));
            }
            println!("{line}");
            Ok(true)
        }
        WatchEvent::MessageChanged {
            session_id,
            message_id,
            session_seq,
            change,
            actor_type,
        } => {
            println!(
                ":: message_event session_id={} message_id={} seq={} actor_type={} change={}",
                session_id, message_id, session_seq, actor_type, change
            );
            let next = SessionWatchSnapshot::load(client, config, &session_id)?;
            for message in next.new_messages_since(snapshot) {
                println!(
                    ":: message id={} seq={} actor={}:{} state={}",
                    message.id,
                    message.session_seq,
                    message.actor_type,
                    message.actor_id,
                    message.state
                );
                println!(":: content_begin");
                println!("{}", message.content_text);
                println!(":: content_end");
            }
            for effect in next.new_effects_since(snapshot) {
                let mut line = format!(
                    ":: effect id={} type={} status={} hook={}",
                    effect.id, effect.effect_type, effect.status, effect.source_hook_id
                );
                if let Some(result_ref) = effect.result_ref.as_deref() {
                    line.push_str(&format!(" result_ref={result_ref}"));
                }
                if let Some(error_text) = effect.error_text.as_deref() {
                    line.push_str(&format!(" error={}", preview_text(error_text)));
                }
                println!("{line}");
            }
            *snapshot = next;
            Ok(true)
        }
    }
}

fn print_effects(effects: &[SessionEffect]) -> Result<()> {
    if effects.is_empty() {
        println!("no session effects recorded");
        return Ok(());
    }

    println!("effects: {}", effects.len());
    for effect in effects {
        let mut parts = vec![
            format!("- id: {}", effect.id),
            format!("type: {}", effect.effect_type),
            format!("status: {}", effect.status),
            format!("hook: {}", effect.source_hook_id),
        ];

        if let Some(result_ref) = effect.result_ref.as_deref() {
            parts.push(format!("result_ref: {}", result_ref));
        }

        if let Some(error_text) = effect.error_text.as_deref() {
            parts.push(format!("error: {}", preview_text(error_text)));
        }

        println!("{}", parts.join(" "));
    }
    Ok(())
}

fn preview_text(text: &str) -> String {
    const MAX_PREVIEW_CHARS: usize = 120;

    let normalized = text.split_whitespace().collect::<Vec<_>>().join(" ");

    if normalized.chars().count() <= MAX_PREVIEW_CHARS {
        return normalized;
    }

    normalized
        .chars()
        .take(MAX_PREVIEW_CHARS)
        .collect::<String>()
        + "…"
}

#[derive(Debug, Deserialize, Serialize)]
struct SessionMemoryResponse {
    id: String,
    memory: String,
    updated_at: String,
}

impl SessionMemoryResponse {
    fn into_output(self, session_id: &str) -> SessionMemoryOutput {
        SessionMemoryOutput {
            session_id: session_id.to_owned(),
            memory: self.memory,
            memory_id: Some(self.id),
            updated_at: Some(self.updated_at),
            exists: true,
        }
    }
}

impl SessionMemoryOutput {
    fn empty(session_id: &str) -> Self {
        Self {
            session_id: session_id.to_owned(),
            memory: String::new(),
            memory_id: None,
            updated_at: None,
            exists: false,
        }
    }
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

fn print_memory(memory: &SessionMemoryOutput) -> Result<()> {
    println!("session_id: {}", memory.session_id);
    println!("exists: {}", memory.exists);
    if let Some(memory_id) = memory.memory_id.as_deref() {
        println!("memory_id: {memory_id}");
    }
    if let Some(updated_at) = memory.updated_at.as_deref() {
        println!("updated_at: {updated_at}");
    }
    println!("memory: {}", memory.memory);
    Ok(())
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
    let mut response = send_message_request(client, config, session_id, message, wait)?;
    if !response.status().is_success() {
        return Err(anyhow!("send request failed: {}", response.status()));
    }

    let mut text = String::new();
    let mut output_text = String::new();
    response.read_to_string(&mut text)?;

    for line in text.lines() {
        let Some(event) = parse_send_sse_line(line)? else {
            continue;
        };
        emit_send_event(event, &mut output_text, raw, json)?;
    }

    finish_send_output(session_id, &output_text, raw, json)?;
    Ok(())
}

fn send_message_request(
    client: &reqwest::blocking::Client,
    config: &Config,
    session_id: &str,
    message: &str,
    wait: bool,
) -> Result<reqwest::blocking::Response> {
    let url = format!(
        "{}/api/v1/sessions/{session_id}/send",
        config.base_url.trim_end_matches('/')
    );
    let body = serde_json::json!({"content":[{"type":"text","text":message}]});

    loop {
        let response = client.post(&url).json(&body).send()?;
        if response.status() != StatusCode::CONFLICT || !wait {
            return Ok(response);
        }
        thread::sleep(Duration::from_millis(350));
    }
}

fn parse_send_sse_line(line: &str) -> Result<Option<SseEvent>> {
    let Some(payload) = parse_sse_data_payload(line) else {
        return Ok(None);
    };

    if let Ok(error) = serde_json::from_str::<SseErrorEnvelope>(payload) {
        return Err(anyhow!(
            "send stream failed: {}: {}",
            error.error.code,
            error.error.message
        ));
    }

    Ok(serde_json::from_str::<SseEvent>(payload).ok())
}

fn emit_send_event(event: SseEvent, output_text: &mut String, raw: bool, json: bool) -> Result<()> {
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

    Ok(())
}

fn finish_send_output(session_id: &str, output_text: &str, raw: bool, json: bool) -> Result<()> {
    if raw {
        return Ok(());
    }

    if json {
        output::json(&ChatOutput {
            session_id,
            output_text,
        })?;
    } else {
        println!();
    }

    Ok(())
}

fn parse_sse_data_payload(line: &str) -> Option<&str> {
    let payload = line.strip_prefix("data: ")?;
    if payload.is_empty() || payload == "[DONE]" {
        return None;
    }

    Some(payload)
}

fn receive_watch_line(
    rx: &mpsc::Receiver<Result<Option<String>, String>>,
    idle_ms: u64,
    session_id: &str,
) -> Result<Option<String>> {
    let received = if idle_ms > 0 {
        match rx.recv_timeout(Duration::from_millis(idle_ms)) {
            Ok(item) => item,
            Err(mpsc::RecvTimeoutError::Timeout) => {
                println!(":: watch session_id={} status=idle_timeout", session_id);
                return Ok(None);
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => return Ok(None),
        }
    } else {
        match rx.recv() {
            Ok(item) => item,
            Err(_) => return Ok(None),
        }
    };

    received
        .map_err(|err| anyhow!("watch stream failed: {err}"))
}

fn parse_watch_sse_line(line: &str) -> Result<Option<WatchEvent>> {
    let Some(payload) = parse_sse_data_payload(line) else {
        return Ok(None);
    };

    if let Ok(error) = serde_json::from_str::<SseErrorEnvelope>(payload) {
        return Err(anyhow!(
            "watch stream failed: {}: {}",
            error.error.code,
            error.error.message
        ));
    }

    Ok(serde_json::from_str::<WatchEvent>(payload).ok())
}

#[cfg(test)]
mod tests {
    use super::{
        SessionMemoryOutput, SessionMemoryResponse, WatchEvent, parse_send_sse_line,
        parse_sse_data_payload, parse_watch_sse_line, preview_text,
    };

    #[test]
    fn session_memory_output_serializes_empty_state() {
        let value = serde_json::to_value(SessionMemoryOutput::empty("session-123")).unwrap();

        assert_eq!(value["session_id"], "session-123");
        assert_eq!(value["memory"], "");
        assert_eq!(value["exists"], false);
        assert!(value.get("memory_id").is_none());
        assert!(value.get("updated_at").is_none());
    }

    #[test]
    fn session_memory_output_preserves_identity_fields() {
        let response = SessionMemoryResponse {
            id: "memory-123".into(),
            memory: "stored memory".into(),
            updated_at: "2026-04-08T00:00:00Z".into(),
        };

        let value = serde_json::to_value(response.into_output("session-123")).unwrap();

        assert_eq!(value["session_id"], "session-123");
        assert_eq!(value["memory_id"], "memory-123");
        assert_eq!(value["memory"], "stored memory");
        assert_eq!(value["updated_at"], "2026-04-08T00:00:00Z");
        assert_eq!(value["exists"], true);
    }

    #[test]
    fn preview_text_flattens_whitespace() {
        assert_eq!(
            preview_text("hello\n\nworld\tfrom cli"),
            "hello world from cli"
        );
    }

    #[test]
    fn preview_text_truncates_long_content() {
        let input = "x".repeat(140);
        let preview = preview_text(&input);

        assert_eq!(preview.chars().count(), 121);
        assert!(preview.ends_with('…'));
    }

    #[test]
    fn parse_sse_data_payload_skips_non_data_and_done_lines() {
        assert_eq!(parse_sse_data_payload("event: ping"), None);
        assert_eq!(parse_sse_data_payload("data: "), None);
        assert_eq!(parse_sse_data_payload("data: [DONE]"), None);
        assert_eq!(parse_sse_data_payload("data: {\"ok\":true}"), Some("{\"ok\":true}"));
    }

    #[test]
    fn parse_send_sse_line_parses_delta_events() {
        let event = parse_send_sse_line(
            "data: {\"type\":\"response.output_text.delta\",\"delta\":\"hello\"}",
        )
        .unwrap();

        match event {
            Some(super::SseEvent::Delta { delta }) => assert_eq!(delta, "hello"),
            other => panic!("unexpected event: {other:?}"),
        }
    }

    #[test]
    fn parse_watch_sse_line_parses_watch_events() {
        let event = parse_watch_sse_line(
            "data: {\"type\":\"state_changed\",\"session_id\":\"session-123\",\"state\":\"running\"}",
        )
        .unwrap();

        match event {
            Some(WatchEvent::StateChanged { session_id, state }) => {
                assert_eq!(session_id, "session-123");
                assert_eq!(state, "running");
            }
            other => panic!("unexpected event: {other:?}"),
        }
    }
}
