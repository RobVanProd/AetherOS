use std::io::{Read, Write};
use std::net::TcpListener;
use std::os::unix::net::UnixListener;
use std::path::Path;

use serde::Serialize;

#[derive(Serialize)]
struct HealthResponse {
    ok: bool,
    service: &'static str,
    version: &'static str,
}

fn write_http_json(stream: &mut dyn Write, status: &str, body: &str) -> anyhow::Result<()> {
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

fn handle_conn(stream: &mut (impl Read + Write)) -> anyhow::Result<()> {
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

enum Listener {
    Unix(UnixListener),
    Tcp(TcpListener),
}

fn main() -> anyhow::Result<()> {
    let tcp_port = std::env::var("AETHERD_TCP_PORT").ok()
        .and_then(|p| p.parse::<u16>().ok());

    let listener = if let Some(port) = tcp_port {
        // TCP mode (forced or fallback)
        let l = TcpListener::bind(format!("0.0.0.0:{}", port))?;
        eprintln!("aetherd listening on tcp://0.0.0.0:{}", port);
        Listener::Tcp(l)
    } else {
        // Try Unix socket first, fall back to TCP on ENOSYS
        let socket_path =
            std::env::var("AETHERD_SOCKET").unwrap_or_else(|_| "/tmp/aetherd.sock".to_string());
        if Path::new(&socket_path).exists() {
            std::fs::remove_file(&socket_path)?;
        }

        match UnixListener::bind(&socket_path) {
            Ok(l) => {
                eprintln!("aetherd listening on unix://{}", socket_path);
                Listener::Unix(l)
            }
            Err(e) if e.raw_os_error() == Some(38) => {
                // ENOSYS — kernel lacks AF_UNIX, fall back to TCP
                let port = 9101u16;
                eprintln!("aetherd: Unix sockets unavailable (ENOSYS), falling back to tcp://0.0.0.0:{}", port);
                let l = TcpListener::bind(format!("0.0.0.0:{}", port))?;
                Listener::Tcp(l)
            }
            Err(e) => return Err(e.into()),
        }
    };

    eprintln!("  audit log: {}/audit.jsonl",
        std::env::var("AETHER_LOG_DIR").unwrap_or_else(|_| "/tmp/aether_logs".to_string()));

    match listener {
        Listener::Unix(l) => {
            for conn in l.incoming() {
                match conn {
                    Ok(mut stream) => {
                        if let Err(err) = handle_conn(&mut stream) {
                            eprintln!("aetherd error: {err:?}");
                        }
                    }
                    Err(err) => eprintln!("aetherd accept error: {err:?}"),
                }
            }
        }
        Listener::Tcp(l) => {
            for conn in l.incoming() {
                match conn {
                    Ok(mut stream) => {
                        if let Err(err) = handle_conn(&mut stream) {
                            eprintln!("aetherd error: {err:?}");
                        }
                    }
                    Err(err) => eprintln!("aetherd accept error: {err:?}"),
                }
            }
        }
    }

    Ok(())
}
