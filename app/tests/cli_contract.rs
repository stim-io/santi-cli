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
            status: "200 OK",
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
