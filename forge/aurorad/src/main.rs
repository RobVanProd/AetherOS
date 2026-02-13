use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::Path;

use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct HealthResponse {
    ok: bool,
    service: &'static str,
    version: &'static str,
}

#[derive(Deserialize)]
struct JobRequest {
    job_type: Option<String>,
    #[serde(flatten)]
    params: serde_json::Value,
}

#[derive(Serialize)]
struct JobResponse {
    ok: bool,
    job_id: String,
    job_type: String,
    result: serde_json::Value,
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

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

/// Send an HTTP request and read the response body.
fn send_http_request(stream: &mut dyn Write, reader: &mut dyn Read, method: &str, path: &str, body: &str) -> anyhow::Result<String> {
    let request = if body.is_empty() {
        format!("{method} {path} HTTP/1.1\r\nHost: localhost\r\n\r\n")
    } else {
        format!(
            "{method} {path} HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
            body.len()
        )
    };

    stream.write_all(request.as_bytes())?;

    let mut response = String::new();
    reader.read_to_string(&mut response)?;

    if let Some(idx) = response.find("\r\n\r\n") {
        Ok(response[idx + 4..].to_string())
    } else {
        Ok(response)
    }
}

/// Forward an HTTP request to cfcd via Unix socket or TCP.
fn forward_to_cfcd(method: &str, path: &str, body: &str) -> anyhow::Result<String> {
    // Check for TCP host (CFCD_HOST=host:port)
    if let Ok(host) = std::env::var("CFCD_HOST") {
        let mut stream = TcpStream::connect(&host)?;
        let mut reader = stream.try_clone()?;
        return send_http_request(&mut stream, &mut reader, method, path, body);
    }

    // Fall back to Unix socket
    let cfcd_socket =
        std::env::var("CFCD_SOCKET").unwrap_or_else(|_| "/tmp/cfcd.sock".to_string());
    let mut stream = UnixStream::connect(&cfcd_socket)?;
    let mut reader = stream.try_clone()?;
    send_http_request(&mut stream, &mut reader, method, path, body)
}

/// Forward a request to the brain server at a specific path.
fn forward_to_brain_path(path: &str, body: &str) -> anyhow::Result<String> {
    let host = std::env::var("BRAIN_HOST").unwrap_or_else(|_| "10.0.2.2:9200".to_string());
    let mut stream = TcpStream::connect(&host)?;
    // Brain queries can take 30+ seconds (LLM latency)
    stream.set_read_timeout(Some(std::time::Duration::from_secs(60)))?;
    stream.set_write_timeout(Some(std::time::Duration::from_secs(5)))?;

    let request = format!(
        "POST {path} HTTP/1.1\r\nHost: {host}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(request.as_bytes())?;

    // Read full response (may be large)
    let mut response = Vec::new();
    let mut buf = [0u8; 4096];
    loop {
        match stream.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => response.extend_from_slice(&buf[..n]),
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock || e.kind() == std::io::ErrorKind::TimedOut => break,
            Err(e) => return Err(e.into()),
        }
    }

    let resp_str = String::from_utf8_lossy(&response).to_string();
    if let Some(idx) = resp_str.find("\r\n\r\n") {
        Ok(resp_str[idx + 4..].to_string())
    } else {
        Ok(resp_str)
    }
}

/// Route job types to cfcd endpoints.
fn route_job_to_cfcd(job_type: &str, params: &serde_json::Value) -> anyhow::Result<String> {
    let (method, path) = match job_type {
        "predict_next_state" => ("POST", "/v0/predict"),
        "encode_state" => ("POST", "/v0/encode_state"),
        "introspect" => ("GET", "/v0/introspect"),
        "trigger_learning" => ("POST", "/v0/update_weights"),
        "enable_learning" => ("POST", "/v0/learning/enable"),
        "disable_learning" => ("POST", "/v0/learning/disable"),
        "save_weights" => ("POST", "/v0/weights/save"),
        _ => ("POST", "/v0/predict"), // default
    };

    let body = if method == "GET" {
        String::new()
    } else {
        serde_json::to_string(params)?
    };

    forward_to_cfcd(method, path, &body)
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
            service: "aurorad",
            version: env!("CARGO_PKG_VERSION"),
        })?;
        return write_http_json(stream, "200 OK", &body);
    }

    // Forward jobs to cfcd
    if method == "POST" && path == "/v0/jobs" {
        let body_str = parse_body(&req);
        let jr: JobRequest =
            serde_json::from_str(body_str).unwrap_or(JobRequest {
                job_type: None,
                params: serde_json::json!({}),
            });
        let jt = jr
            .job_type
            .unwrap_or_else(|| "predict_next_state".to_string());

        // Route brain jobs to brain server, everything else to cfcd
        let result_value = if jt == "brain" || jt == "brain_proactive" || jt == "brain_dashboard" {
            let brain_path = match jt.as_str() {
                "brain_proactive" => "/v0/brain/proactive",
                "brain_dashboard" => "/v0/brain/dashboard",
                _ => "/v0/brain",
            };
            let brain_body = serde_json::to_string(&jr.params)?;
            match forward_to_brain_path(brain_path, &brain_body) {
                Ok(resp_body) => {
                    serde_json::from_str(&resp_body).unwrap_or(serde_json::json!({"raw": resp_body}))
                }
                Err(e) => {
                    eprintln!("brain forward failed: {e:?} (is brain_server running?)");
                    serde_json::json!({"error": format!("brain unavailable: {e}"), "ok": false})
                }
            }
        } else {
            // Forward to cfcd
            match route_job_to_cfcd(&jt, &jr.params) {
                Ok(resp_body) => {
                    serde_json::from_str(&resp_body).unwrap_or(serde_json::json!({"raw": resp_body}))
                }
                Err(e) => {
                    eprintln!("cfcd forward failed: {e:?} (is cfcd running?)");
                    serde_json::json!({"error": format!("cfcd unavailable: {e}"), "mocked": true})
                }
            }
        };

        let resp = JobResponse {
            ok: true,
            job_id: format!("job_{}", now_secs()),
            job_type: jt,
            result: result_value,
        };

        let body = serde_json::to_string(&resp)?;
        return write_http_json(stream, "200 OK", &body);
    }

    // Proxy model endpoints directly to cfcd
    if path.starts_with("/v0/model/") || path.starts_with("/v0/cfcd/") {
        let cfcd_path = path.replacen("/v0/model/", "/v0/", 1)
            .replacen("/v0/cfcd/", "/v0/", 1);
        let body_str = parse_body(&req);

        match forward_to_cfcd(method, &cfcd_path, body_str) {
            Ok(resp_body) => return write_http_json(stream, "200 OK", &resp_body),
            Err(e) => {
                let err = serde_json::json!({"ok": false, "error": format!("cfcd: {e}")});
                return write_http_json(stream, "502 Bad Gateway", &err.to_string());
            }
        }
    }

    let body = "{\"ok\":false,\"error\":\"not_found\"}";
    write_http_json(stream, "404 Not Found", body)
}

enum Listener {
    Unix(UnixListener),
    Tcp(TcpListener),
}

fn main() -> anyhow::Result<()> {
    let tcp_port = std::env::var("AURORAD_TCP_PORT").ok()
        .and_then(|p| p.parse::<u16>().ok());

    let listener = if let Some(port) = tcp_port {
        let l = TcpListener::bind(format!("0.0.0.0:{}", port))?;
        eprintln!("aurorad listening on tcp://0.0.0.0:{}", port);
        Listener::Tcp(l)
    } else {
        let socket_path =
            std::env::var("AURORAD_SOCKET").unwrap_or_else(|_| "/tmp/aurorad.sock".to_string());
        if Path::new(&socket_path).exists() {
            std::fs::remove_file(&socket_path)?;
        }

        match UnixListener::bind(&socket_path) {
            Ok(l) => {
                eprintln!("aurorad listening on unix://{}", socket_path);
                Listener::Unix(l)
            }
            Err(e) if e.raw_os_error() == Some(38) => {
                let port = 9102u16;
                eprintln!("aurorad: Unix sockets unavailable (ENOSYS), falling back to tcp://0.0.0.0:{}", port);
                let l = TcpListener::bind(format!("0.0.0.0:{}", port))?;
                Listener::Tcp(l)
            }
            Err(e) => return Err(e.into()),
        }
    };

    if let Ok(host) = std::env::var("CFCD_HOST") {
        eprintln!("  cfcd forwarding via TCP: {}", host);
    } else {
        let sock = std::env::var("CFCD_SOCKET").unwrap_or_else(|_| "/tmp/cfcd.sock".to_string());
        eprintln!("  cfcd forwarding via Unix: {}", sock);
    }

    let brain_host = std::env::var("BRAIN_HOST").unwrap_or_else(|_| "10.0.2.2:9200".to_string());
    eprintln!("  brain forwarding via TCP: {}", brain_host);

    match listener {
        Listener::Unix(l) => {
            for conn in l.incoming() {
                match conn {
                    Ok(mut stream) => {
                        if let Err(err) = handle_conn(&mut stream) {
                            eprintln!("aurorad error: {err:?}");
                        }
                    }
                    Err(err) => eprintln!("aurorad accept error: {err:?}"),
                }
            }
        }
        Listener::Tcp(l) => {
            for conn in l.incoming() {
                match conn {
                    Ok(mut stream) => {
                        if let Err(err) = handle_conn(&mut stream) {
                            eprintln!("aurorad error: {err:?}");
                        }
                    }
                    Err(err) => eprintln!("aurorad accept error: {err:?}"),
                }
            }
        }
    }

    Ok(())
}
