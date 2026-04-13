use anyhow::Result;

use super::{
    SessionCompactResponse, SessionEffect, SessionMemoryOutput, SessionMessage, SessionResponse,
    SoulMemoryResponse, SoulResponse,
};

pub(super) fn print_session(session: &SessionResponse) -> Result<()> {
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

pub(super) fn print_compact(compact: &SessionCompactResponse) -> Result<()> {
    println!("id: {}", compact.id);
    println!("turn_id: {}", compact.turn_id);
    println!("summary: {}", compact.summary);
    println!("start_session_seq: {}", compact.start_session_seq);
    println!("end_session_seq: {}", compact.end_session_seq);
    println!("created_at: {}", compact.created_at);
    Ok(())
}

pub(super) fn print_compacts(compacts: &[SessionCompactResponse]) -> Result<()> {
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

pub(super) fn print_messages(messages: &[SessionMessage]) -> Result<()> {
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

pub(super) fn print_soul(soul: &SoulResponse) -> Result<()> {
    println!("id: {}", soul.id);
    println!("memory: {}", soul.memory);
    println!("created_at: {}", soul.created_at);
    println!("updated_at: {}", soul.updated_at);
    Ok(())
}

pub(super) fn print_soul_memory(memory: &SoulMemoryResponse) -> Result<()> {
    println!("id: {}", memory.id);
    println!("memory: {}", memory.memory);
    println!("updated_at: {}", memory.updated_at);
    Ok(())
}

pub(super) fn print_effects(effects: &[SessionEffect]) -> Result<()> {
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

pub(super) fn print_memory(memory: &SessionMemoryOutput) -> Result<()> {
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

pub(super) fn preview_text(text: &str) -> String {
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
