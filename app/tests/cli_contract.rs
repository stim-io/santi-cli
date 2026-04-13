use std::{
    io::{Read, Write},
    net::TcpListener,
    thread,
    time::Duration,
};

use assert_cmd::Command;

struct ExpectedRequest<'a> {
    method: &'a str,
    path: &'a str,
    status: &'a str,
    content_type: &'a str,
    body: &'a str,
}

fn wrapped_effects_response_body() -> &'static str {
    r#"{"effects":[{"id":"effect_123","session_id":"sess_test","effect_type":"hook_fork_handoff","idempotency_key":"hook_fork_handoff:auto-fork-handoff:turn_123:6","status":"completed","source_hook_id":"auto-fork-handoff","source_turn_id":"turn_123","result_ref":"sess_child","error_text":null,"created_at":"2026-04-09T00:00:00Z","updated_at":"2026-04-09T00:00:01Z"}]}"#
}

fn wrapped_messages_response_body(messages: &[&str]) -> String {
    let body = messages
        .iter()
        .enumerate()
        .map(|(index, text)| {
            let session_seq = index as i64 + 1;
            let actor_type = if index % 2 == 0 { "account" } else { "soul" };
            let actor_id = if index % 2 == 0 {
                "account_local"
            } else {
                "soul_default"
            };
            format!(
                "{{\"id\":\"msg_{session_seq}\",\"actor_type\":\"{actor_type}\",\"actor_id\":\"{actor_id}\",\"session_seq\":{session_seq},\"content_text\":\"{text}\",\"state\":\"fixed\",\"created_at\":\"2026-04-09T00:00:0{session_seq}Z\"}}"
            )
        })
        .collect::<Vec<_>>()
        .join(",");

    format!("{{\"messages\":[{body}]}}")
}

fn wrapped_watch_snapshot_response_body(messages: &[&str]) -> String {
    let messages = wrapped_messages_response_body(messages);
    let messages = messages
        .trim_start_matches("{\"messages\":")
        .trim_end_matches('}');
    format!(
        "{{\"session_id\":\"sess_test\",\"latest_seq\":{},\"messages\":{},\"effects\":[]}}",
        messages.matches("\"id\"").count(),
        messages
    )
}

fn watch_sse_body() -> &'static str {
    concat!(
        "data: {\"type\":\"connected\",\"session_id\":\"sess_test\",\"latest_seq\":1}\n\n",
        "data: {\"type\":\"state_changed\",\"session_id\":\"sess_test\",\"state\":\"running\"}\n\n",
        "data: {\"type\":\"activity_changed\",\"session_id\":\"sess_test\",\"activity\":\"send\",\"state\":\"started\",\"label\":null}\n\n",
        "data: {\"type\":\"message_changed\",\"session_id\":\"sess_test\",\"message_id\":\"msg_2\",\"session_seq\":2,\"change\":\"finalized\",\"actor_type\":\"soul\"}\n\n"
    )
}

fn spawn_stub_server(requests: Vec<ExpectedRequest<'static>>) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();

    thread::spawn(move || {
        for expected in requests {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(3)))
                .unwrap();

            let request = read_http_request(&mut stream);
            let mut lines = request.lines();
            let request_line = lines.next().unwrap_or_default();
            let mut parts = request_line.split_whitespace();
            let method = parts.next().unwrap_or_default();
            let path = parts.next().unwrap_or_default();

            assert_eq!(method, expected.method);
            assert_eq!(path, expected.path);

            write!(
                stream,
                "HTTP/1.1 {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                expected.status,
                expected.content_type,
                expected.body.len(),
                expected.body
            )
            .unwrap();
            stream.flush().unwrap();
        }
    });

    format!("http://{}", addr)
}

fn read_http_request(stream: &mut std::net::TcpStream) -> String {
    let mut buffer = Vec::new();
    let mut chunk = [0_u8; 1024];
    let mut header_end = None;
    let mut content_length = 0usize;

    loop {
        let read = stream.read(&mut chunk).unwrap();
        if read == 0 {
            break;
        }
        buffer.extend_from_slice(&chunk[..read]);

        if header_end.is_none()
            && let Some(pos) = buffer.windows(4).position(|window| window == b"\r\n\r\n")
        {
            header_end = Some(pos + 4);
            let headers = String::from_utf8_lossy(&buffer[..pos + 4]);
            for line in headers.lines() {
                if let Some(value) = line.strip_prefix("Content-Length:") {
                    content_length = value.trim().parse().unwrap();
                }
            }
        }

        if let Some(end) = header_end
            && buffer.len() >= end + content_length
        {
            break;
        }
    }

    String::from_utf8(buffer).unwrap()
}

#[test]
fn health_human_uses_stdout_only() {
    let base_url = spawn_stub_server(vec![ExpectedRequest {
        method: "GET",
        path: "/api/v1/health",
        status: "200 OK",
        content_type: "application/json",
        body: r#"{"status":"ok"}"#,
    }]);

    let output = Command::cargo_bin("santi-cli")
        .unwrap()
        .args(["--base-url", &base_url, "health"])
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stderr), "");
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        format!("status: ok\nbase_url: {base_url}\n")
    );
}

#[test]
fn health_json_is_stdout_json_only() {
    let base_url = spawn_stub_server(vec![ExpectedRequest {
        method: "GET",
        path: "/api/v1/health",
        status: "200 OK",
        content_type: "application/json",
        body: r#"{"status":"ok"}"#,
    }]);

    let output = Command::cargo_bin("santi-cli")
        .unwrap()
        .args(["--base-url", &base_url, "--json", "health"])
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stderr), "");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(parsed["status"], "ok");
    assert_eq!(parsed["base_url"], base_url);
}

#[test]
fn chat_human_auto_create_puts_session_on_stderr() {
    let base_url = spawn_stub_server(vec![
        ExpectedRequest {
            method: "POST",
            path: "/api/v1/sessions",
            status: "201 Created",
            content_type: "application/json",
            body: r#"{"id":"sess_test","parent_session_id":null,"fork_point":null,"created_at":"2026-04-08T00:00:00Z"}"#,
        },
        ExpectedRequest {
            method: "POST",
            path: "/api/v1/sessions/sess_test/send",
            status: "200 OK",
            content_type: "text/event-stream",
            body: concat!(
                "data: {\"type\":\"response.output_text.delta\",\"delta\":\"Hello\"}\n\n",
                "data: {\"type\":\"response.completed\"}\n\n",
                "data: [DONE]\n\n"
            ),
        },
    ]);

    let output = Command::cargo_bin("santi-cli")
        .unwrap()
        .args(["--base-url", &base_url, "chat", "hello"])
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout), "Hello\n");
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "session_id: sess_test\n"
    );
}

#[test]
fn chat_raw_streams_events_without_stderr_noise() {
    let base_url = spawn_stub_server(vec![ExpectedRequest {
        method: "POST",
        path: "/api/v1/sessions/sess_test/send",
        status: "200 OK",
        content_type: "text/event-stream",
        body: concat!(
            "data: {\"type\":\"response.output_text.delta\",\"delta\":\"Hi\"}\n\n",
            "data: {\"type\":\"response.completed\"}\n\n",
            "data: [DONE]\n\n"
        ),
    }]);

    let output = Command::cargo_bin("santi-cli")
        .unwrap()
        .args([
            "--base-url",
            &base_url,
            "chat",
            "--session",
            "sess_test",
            "--raw",
            "hello",
        ])
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stderr), "");
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "{\"type\":\"response.output_text.delta\",\"delta\":\"Hi\"}\n{\"type\":\"response.completed\"}\n"
    );
}

#[test]
fn chat_rejects_json_and_raw_together() {
    let output = Command::cargo_bin("santi-cli")
        .unwrap()
        .args(["--json", "chat", "--raw", "hello"])
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("--raw"));
    assert!(stderr.contains("--json"));
}

#[test]
fn session_send_rejects_json_and_raw_together() {
    let output = Command::cargo_bin("santi-cli")
        .unwrap()
        .args(["--json", "session", "send", "sess_test", "--raw", "hello"])
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("--raw"));
    assert!(stderr.contains("--json"));
}

#[test]
fn session_watch_rejects_json_mode() {
    let output = Command::cargo_bin("santi-cli")
        .unwrap()
        .args(["--json", "session", "watch", "sess_test"])
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("session watch"));
    assert!(stderr.contains("--json"));
}

#[test]
fn session_watch_uses_watch_snapshot_and_sse_stream() {
    let base_url = spawn_stub_server(vec![
        ExpectedRequest {
            method: "GET",
            path: "/api/v1/sessions/sess_test/watch-snapshot",
            status: "200 OK",
            content_type: "application/json",
            body: Box::leak(wrapped_watch_snapshot_response_body(&["start task"]).into_boxed_str()),
        },
        ExpectedRequest {
            method: "GET",
            path: "/api/v1/sessions/sess_test/watch",
            status: "200 OK",
            content_type: "text/event-stream",
            body: watch_sse_body(),
        },
        ExpectedRequest {
            method: "GET",
            path: "/api/v1/sessions/sess_test/watch-snapshot",
            status: "200 OK",
            content_type: "application/json",
            body: Box::leak(
                wrapped_watch_snapshot_response_body(&["start task", "done"]).into_boxed_str(),
            ),
        },
    ]);

    let output = Command::cargo_bin("santi-cli")
        .unwrap()
        .args([
            "--base-url",
            &base_url,
            "session",
            "watch",
            "sess_test",
            "--idle-ms",
            "10",
        ])
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stderr), "");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains(":: watch session_id=sess_test status=started transport=sse idle_ms=10")
    );
    assert!(stdout.contains(":: watch session_id=sess_test status=connected last_seq=1"));
    assert!(stdout.contains(":: state session_id=sess_test state=running"));
    assert!(stdout.contains(":: activity session_id=sess_test activity=send state=started"));
    assert!(stdout.contains(":: message id=msg_2 seq=2 actor=soul:soul_default state=fixed"));
    assert!(stdout.contains(":: content_begin\ndone\n:: content_end"));
}

#[test]
fn session_effects_json_decodes_wrapped_effects_response() {
    let base_url = spawn_stub_server(vec![ExpectedRequest {
        method: "GET",
        path: "/api/v1/sessions/sess_test/effects",
        status: "200 OK",
        content_type: "application/json",
        body: wrapped_effects_response_body(),
    }]);

    let output = Command::cargo_bin("santi-cli")
        .unwrap()
        .args([
            "--base-url",
            &base_url,
            "--json",
            "session",
            "effects",
            "sess_test",
        ])
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stderr), "");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(parsed[0]["id"], "effect_123");
    assert_eq!(parsed[0]["effect_type"], "hook_fork_handoff");
    assert_eq!(parsed[0]["result_ref"], "sess_child");
}

#[test]
fn session_effects_human_prints_effect_summary() {
    let base_url = spawn_stub_server(vec![ExpectedRequest {
        method: "GET",
        path: "/api/v1/sessions/sess_test/effects",
        status: "200 OK",
        content_type: "application/json",
        body: wrapped_effects_response_body(),
    }]);

    let output = Command::cargo_bin("santi-cli")
        .unwrap()
        .args(["--base-url", &base_url, "session", "effects", "sess_test"])
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stderr), "");
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "effects: 1\n- id: effect_123 type: hook_fork_handoff status: completed hook: auto-fork-handoff result_ref: sess_child\n"
    );
}

#[test]
fn session_send_fails_on_sse_error_payload() {
    let base_url = spawn_stub_server(vec![ExpectedRequest {
        method: "POST",
        path: "/api/v1/sessions/sess_test/send",
        status: "200 OK",
        content_type: "text/event-stream",
        body: concat!(
            "data: {\"error\":{\"code\":\"conflict\",\"message\":\"session send already in progress\"}}\n\n",
            "data: [DONE]\n\n"
        ),
    }]);

    let output = Command::cargo_bin("santi-cli")
        .unwrap()
        .args([
            "--base-url",
            &base_url,
            "--json",
            "session",
            "send",
            "sess_test",
            "hello",
        ])
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout), "");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("send stream failed"));
    assert!(stderr.contains("conflict"));
}
