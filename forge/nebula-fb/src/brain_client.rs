/// Brain client — adapted from nebula-tui for framebuffer variant.

use std::io::{Read, Write};
use std::net::TcpStream;
use std::os::unix::net::UnixStream;
use std::time::Duration;

use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
pub struct Widget {
    #[serde(rename = "type")]
    pub widget_type: String,
    pub title: String,
    #[serde(default)]
    pub lines: Vec<String>,
}

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

/// Dashboard layout response from the brain server.
#[derive(Clone, Debug, Deserialize)]
pub struct DashboardResponse {
    #[serde(default)]
    pub ok: bool,
    #[serde(default)]
    pub greeting: String,
    #[serde(default)]
    pub subtitle: String,
    #[serde(default)]
    pub cards: Vec<serde_json::Value>,
    #[serde(default)]
    pub latency_ms: u64,
}

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

fn http_post_aurorad(body_str: &str, timeout_secs: u64) -> Result<String, String> {
    let addr = aurorad_addr();
    let request = format!(
        "POST /v0/jobs HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body_str.len(), body_str
    );

    let resp_body = if addr.contains(':') && !addr.starts_with('/') {
        let mut stream = TcpStream::connect(&addr).map_err(|e| format!("connect: {e}"))?;
        stream.set_read_timeout(Some(Duration::from_secs(timeout_secs))).ok();
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
        stream.set_read_timeout(Some(Duration::from_secs(timeout_secs))).ok();
        stream.write_all(request.as_bytes()).map_err(|e| format!("write: {e}"))?;
        let mut resp = String::new();
        stream.read_to_string(&mut resp).map_err(|e| format!("read: {e}"))?;
        extract_body(&resp)
    };

    Ok(resp_body)
}

/// Send a brain query via aurorad.
pub fn query_brain(input: &str) -> Result<BrainResponse, String> {
    let body = serde_json::json!({
        "job_type": "brain",
        "input": input
    });
    let resp_body = http_post_aurorad(&body.to_string(), 90)?;

    if let Ok(job_resp) = serde_json::from_str::<serde_json::Value>(&resp_body) {
        if let Some(result) = job_resp.get("result") {
            if let Ok(brain) = serde_json::from_value::<BrainResponse>(result.clone()) {
                return Ok(brain);
            }
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
            let raw = serde_json::to_string_pretty(result).unwrap_or(resp_body.clone());
            return Ok(BrainResponse {
                ok: true,
                text: raw,
                widgets: vec![],
                latency_ms: 0,
                error: None,
            });
        }
        if let Some(err) = job_resp.get("error").and_then(|e| e.as_str()) {
            return Err(err.to_string());
        }
    }

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

/// Query the brain for a personalized dashboard layout.
pub fn query_brain_dashboard(
    name: &str,
    interests: &[String],
    cpu: f64,
    mem_pct: f64,
    uptime: &str,
) -> Result<DashboardResponse, String> {
    let body = serde_json::json!({
        "job_type": "brain_dashboard",
        "name": name,
        "interests": interests,
        "telemetry": {
            "cpu": cpu,
            "mem_pct": mem_pct,
            "uptime": uptime,
        }
    });
    let resp_body = http_post_aurorad(&body.to_string(), 45)?;

    if let Ok(job_resp) = serde_json::from_str::<serde_json::Value>(&resp_body) {
        if let Some(result) = job_resp.get("result") {
            if let Ok(dashboard) = serde_json::from_value::<DashboardResponse>(result.clone()) {
                return Ok(dashboard);
            }
        }
    }

    match serde_json::from_str::<DashboardResponse>(&resp_body) {
        Ok(d) => Ok(d),
        Err(e) => Err(format!("parse: {e}")),
    }
}

fn extract_body(resp: &str) -> String {
    if let Some(idx) = resp.find("\r\n\r\n") {
        resp[idx + 4..].to_string()
    } else {
        resp.to_string()
    }
}
