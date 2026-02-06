use std::io::{Read, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::Path;

use serde::Serialize;

#[derive(Serialize)]
struct HealthResponse {
    ok: bool,
    service: &'static str,
    version: &'static str,
}

fn write_http_json(mut stream: UnixStream, status: &str, body: &str) -> anyhow::Result<()> {
    let resp = format!(
        "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
        body.len(),
        body
    );
    stream.write_all(resp.as_bytes())?;
    Ok(())
}

fn parse_body(req: &str) -> &str {
    req.split("\r\n\r\n").nth(1).unwrap_or("")
}

fn append_audit_log(event: &str) {
    let log_dir = std::env::var("AETHER_LOG_DIR")
        .unwrap_or_else(|_| "/tmp/aether_logs".to_string());

    let _ = std::fs::create_dir_all(&log_dir);
    let log_path = format!("{}/audit.jsonl", log_dir);

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let entry = format!("{{\"ts\":{},\"event\":{}}}\n", timestamp, event);

    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
    {
        let _ = f.write_all(entry.as_bytes());
    }
}

fn handle_conn(mut stream: UnixStream) -> anyhow::Result<()> {
    let mut buf = [0u8; 16384];
    let n = stream.read(&mut buf)?;
    let req = String::from_utf8_lossy(&buf[..n]);

    let mut lines = req.lines();
    let first = lines.next().unwrap_or("");
    let mut parts = first.split_whitespace();
    let method = parts.next().unwrap_or("");
    let path = parts.next().unwrap_or("/");

    if method == "GET" && path == "/v0/health" {
        let body = serde_json::to_string(&HealthResponse {
            ok: true,
            service: "aetherd",
            version: env!("CARGO_PKG_VERSION"),
        })?;
        return write_http_json(stream, "200 OK", &body);
    }

    // Audit logging endpoint
    if method == "POST" && path == "/v0/audit" {
        let body_str = parse_body(&req);
        append_audit_log(body_str);
        let resp = "{\"ok\":true,\"logged\":true}";
        return write_http_json(stream, "200 OK", resp);
    }

    // Policy check endpoint (v0: always allow, but log)
    if method == "POST" && path == "/v0/policy/check" {
        let body_str = parse_body(&req);
        append_audit_log(&format!("{{\"type\":\"policy_check\",\"request\":{}}}", body_str));
        let resp = "{\"ok\":true,\"allowed\":true,\"reason\":\"v0_allow_all\"}";
        return write_http_json(stream, "200 OK", resp);
    }

    let body = "{\"ok\":false,\"error\":\"not_found\"}";
    write_http_json(stream, "404 Not Found", body)
}

fn main() -> anyhow::Result<()> {
    let socket_path =
        std::env::var("AETHERD_SOCKET").unwrap_or_else(|_| "/tmp/aetherd.sock".to_string());
    if Path::new(&socket_path).exists() {
        std::fs::remove_file(&socket_path)?;
    }

    let listener = UnixListener::bind(&socket_path)?;
    eprintln!("aetherd listening on unix://{}", socket_path);
    eprintln!("  audit log: {}/audit.jsonl",
        std::env::var("AETHER_LOG_DIR").unwrap_or_else(|_| "/tmp/aether_logs".to_string()));

    for conn in listener.incoming() {
        match conn {
            Ok(stream) => {
                if let Err(err) = handle_conn(stream) {
                    eprintln!("aetherd error: {err:?}");
                }
            }
            Err(err) => eprintln!("aetherd accept error: {err:?}"),
        }
    }

    Ok(())
}
