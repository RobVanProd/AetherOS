use std::io::{Read, Write};
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
}

#[derive(Serialize)]
struct JobResponse {
    ok: bool,
    job_id: String,
    mocked: bool,
    job_type: String,
    result: serde_json::Value,
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
    // naive split; fine for v0 demo.
    req.split("\r\n\r\n").nth(1).unwrap_or("")
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
            service: "aurorad",
            version: env!("CARGO_PKG_VERSION"),
        })?;
        return write_http_json(stream, "200 OK", &body);
    }

    if method == "POST" && path == "/v0/jobs" {
        let body_str = parse_body(&req);
        let jr: JobRequest = serde_json::from_str(body_str).unwrap_or(JobRequest { job_type: None });
        let jt = jr.job_type.unwrap_or_else(|| "predict_next_state".to_string());

        let resp = JobResponse {
            ok: true,
            job_id: format!("job_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs()),
            mocked: true,
            job_type: jt.clone(),
            result: serde_json::json!({"note": "mocked_result", "job_type": jt}),
        };

        let body = serde_json::to_string(&resp)?;
        return write_http_json(stream, "200 OK", &body);
    }

    let body = "{\"ok\":false,\"error\":\"not_found\"}";
    write_http_json(stream, "404 Not Found", body)
}

fn main() -> anyhow::Result<()> {
    let socket_path = std::env::var("AURORAD_SOCKET").unwrap_or_else(|_| "/tmp/aurorad.sock".to_string());
    if Path::new(&socket_path).exists() {
        std::fs::remove_file(&socket_path)?;
    }

    let listener = UnixListener::bind(&socket_path)?;
    eprintln!("aurorad listening on unix://{}", socket_path);

    for conn in listener.incoming() {
        match conn {
            Ok(stream) => {
                if let Err(err) = handle_conn(stream) {
                    eprintln!("aurorad error: {err:?}");
                }
            }
            Err(err) => eprintln!("aurorad accept error: {err:?}"),
        }
    }

    Ok(())
}
