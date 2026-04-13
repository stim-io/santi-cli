use std::{io::Read, thread, time::Duration};

use anyhow::anyhow;
use anyhow::Result;
use reqwest::StatusCode;
use serde::Deserialize;

use crate::{config::Config, output};

use super::ChatOutput;

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub(super) enum SseEvent {
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

pub(super) fn send_message(
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

pub(super) fn parse_send_sse_line(line: &str) -> Result<Option<SseEvent>> {
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

pub(super) fn parse_sse_data_payload(line: &str) -> Option<&str> {
    let payload = line.strip_prefix("data: ")?;
    if payload.is_empty() || payload == "[DONE]" {
        return None;
    }

    Some(payload)
}
