use std::io::{Read, Write};
use std::net::TcpStream;
use std::os::unix::net::UnixStream;
use std::time::Duration;

use serde::Deserialize;

/// Widget from brain response.
#[derive(Clone, Debug, Deserialize)]
pub struct Widget {
    #[serde(rename = "type")]
    pub widget_type: String,
    pub title: String,
    #[serde(default)]
    pub lines: Vec<String>,
}

/// Brain response from the brain server.
#[derive(Clone, Debug, Deserialize)]
pub struct BrainResponse {
    pub ok: bool,
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub widgets: Vec<Widget>,
    #[serde(default)]
    pub latency_ms: u64,
    #[serde(default)]
    pub error: Option<String>,
}

/// How to reach aurorad (which forwards brain queries).
fn aurorad_addr() -> String {
    if let Ok(port) = std::env::var("AURORAD_TCP_PORT") {
        if let Ok(p) = port.parse::<u16>() {
            return format!("127.0.0.1:{}", p);
        }
    }
    if let Ok(host) = std::env::var("AURORAD_HOST") {
        return host;
    }
    "127.0.0.1:9102".to_string()
}

/// Send a brain query via aurorad and return the parsed response.
pub fn query_brain(input: &str) -> Result<BrainResponse, String> {
    let addr = aurorad_addr();
    let body = serde_json::json!({
        "job_type": "brain",
        "input": input
    });
    let body_str = body.to_string();

    let request = format!(
        "POST /v0/jobs HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body_str.len(), body_str
    );

    // Try TCP connection to aurorad
    let resp_body = if addr.contains(':') && !addr.starts_with('/') {
        let mut stream = TcpStream::connect(&addr).map_err(|e| format!("connect: {e}"))?;
        stream.set_read_timeout(Some(Duration::from_secs(90))).ok();
        stream.set_write_timeout(Some(Duration::from_secs(5))).ok();
        stream.write_all(request.as_bytes()).map_err(|e| format!("write: {e}"))?;

        let mut resp = Vec::new();
        let mut buf = [0u8; 4096];
        loop {
            match stream.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => resp.extend_from_slice(&buf[..n]),
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock
                    || e.kind() == std::io::ErrorKind::TimedOut => break,
                Err(e) => return Err(format!("read: {e}")),
            }
        }
        let resp_str = String::from_utf8_lossy(&resp).to_string();
        extract_body(&resp_str)
    } else {
        let mut stream = UnixStream::connect(&addr).map_err(|e| format!("connect: {e}"))?;
        stream.set_read_timeout(Some(Duration::from_secs(90))).ok();
        stream.write_all(request.as_bytes()).map_err(|e| format!("write: {e}"))?;
        let mut resp = String::new();
        stream.read_to_string(&mut resp).map_err(|e| format!("read: {e}"))?;
        extract_body(&resp)
    };

    // Parse the aurorad job response — brain result is nested in "result"
    if let Ok(job_resp) = serde_json::from_str::<serde_json::Value>(&resp_body) {
        if let Some(result) = job_resp.get("result") {
            // The brain response is inside the "result" field
            if let Ok(brain) = serde_json::from_value::<BrainResponse>(result.clone()) {
                return Ok(brain);
            }
            // If it has a "text" field directly
            if let Some(text) = result.get("text").and_then(|t| t.as_str()) {
                let widgets: Vec<Widget> = result.get("widgets")
                    .and_then(|w| serde_json::from_value(w.clone()).ok())
                    .unwrap_or_default();
                return Ok(BrainResponse {
                    ok: true,
                    text: text.to_string(),
                    widgets,
                    latency_ms: 0,
                    error: None,
                });
            }
            // Raw result
            let raw = serde_json::to_string_pretty(result).unwrap_or(resp_body.clone());
            return Ok(BrainResponse {
                ok: true,
                text: raw,
                widgets: vec![],
                latency_ms: 0,
                error: None,
            });
        }
        // Check for error at job level
        if let Some(err) = job_resp.get("error").and_then(|e| e.as_str()) {
            return Err(err.to_string());
        }
    }

    // Try parsing directly as BrainResponse
    match serde_json::from_str::<BrainResponse>(&resp_body) {
        Ok(brain) => Ok(brain),
        Err(_) => Ok(BrainResponse {
            ok: true,
            text: resp_body,
            widgets: vec![],
            latency_ms: 0,
            error: None,
        }),
    }
}

fn extract_body(resp: &str) -> String {
    if let Some(idx) = resp.find("\r\n\r\n") {
        resp[idx + 4..].to_string()
    } else {
        resp.to_string()
    }
}
