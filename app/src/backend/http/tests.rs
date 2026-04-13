use super::{
    SessionMemoryOutput, SessionMemoryResponse,
    send::{SseEvent, parse_send_sse_line, parse_sse_data_payload},
    watch::{WatchEvent, parse_watch_sse_line},
};
use crate::backend::http::render::preview_text;

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
    assert_eq!(
        parse_sse_data_payload("data: {\"ok\":true}"),
        Some("{\"ok\":true}")
    );
}

#[test]
fn parse_send_sse_line_parses_delta_events() {
    let event =
        parse_send_sse_line("data: {\"type\":\"response.output_text.delta\",\"delta\":\"hello\"}")
            .unwrap();

    match event {
        Some(SseEvent::Delta { delta }) => assert_eq!(delta, "hello"),
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
