use std::{
    collections::BTreeSet,
    io::{BufRead, BufReader},
    sync::mpsc,
    thread,
    time::Duration,
};

use anyhow::anyhow;
use anyhow::Result;
use serde::Deserialize;

use crate::{cli::SessionWatchCommand, config::Config};

use super::{render::preview_text, send::parse_sse_data_payload, SessionEffect, SessionMessage};

#[derive(Debug, Deserialize)]
struct SseErrorEnvelope {
    error: SseErrorBody,
}

#[derive(Debug, Deserialize)]
struct SseErrorBody {
    code: String,
    message: String,
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
pub(super) enum WatchEvent {
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

pub(super) fn watch_session(
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

    received.map_err(|err| anyhow!("watch stream failed: {err}"))
}

pub(super) fn parse_watch_sse_line(line: &str) -> Result<Option<WatchEvent>> {
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
