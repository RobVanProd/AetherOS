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

/// Proactive context sent to the brain for insight generation.
#[derive(Clone, Debug, serde::Serialize)]
pub struct ProactiveContext {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub telemetry: Option<TelemetryContext>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub world_model: Option<WorldModelContext>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub recent_alerts: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_activity: Option<UserActivityContext>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tasks: Option<TaskContext>,
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct TelemetryContext {
    pub cpu: f64,
    pub mem_pct: f64,
    pub uptime: String,
    pub procs: u32,
    pub network: String,
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct WorldModelContext {
    pub prediction_error: f64,
    pub trend: String,
    pub learning_enabled: bool,
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct UserActivityContext {
    pub last_query: String,
    pub session_duration: String,
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct TaskContext {
    pub active: usize,
    pub completed: usize,
}

/// Brain proactive response.
#[derive(Clone, Debug, Deserialize)]
pub struct ProactiveResponse {
    #[serde(default)]
    pub has_insight: bool,
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub widgets: Vec<Widget>,
    #[serde(default)]
    pub priority: String,
    #[serde(default)]
    pub category: String,
}

/// Query the brain's proactive endpoint for insights.
pub fn query_brain_proactive(context: &ProactiveContext) -> Result<ProactiveResponse, String> {
    let addr = aurorad_addr();
    let body = serde_json::json!({
        "job_type": "brain_proactive",
        "telemetry": context.telemetry,
        "world_model": context.world_model,
        "recent_alerts": context.recent_alerts,
        "user_activity": context.user_activity,
        "tasks": context.tasks,
    });
    let body_str = body.to_string();

    let request = format!(
        "POST /v0/jobs HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body_str.len(), body_str
    );

    let resp_body = if addr.contains(':') && !addr.starts_with('/') {
        let mut stream = TcpStream::connect(&addr).map_err(|e| format!("connect: {e}"))?;
        stream.set_read_timeout(Some(Duration::from_secs(45))).ok();
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
        stream.set_read_timeout(Some(Duration::from_secs(45))).ok();
        stream.write_all(request.as_bytes()).map_err(|e| format!("write: {e}"))?;
        let mut resp = String::new();
        stream.read_to_string(&mut resp).map_err(|e| format!("read: {e}"))?;
        extract_body(&resp)
    };

    // Parse the aurorad job response — proactive result is nested in "result"
    if let Ok(job_resp) = serde_json::from_str::<serde_json::Value>(&resp_body) {
        if let Some(result) = job_resp.get("result") {
            if let Ok(proactive) = serde_json::from_value::<ProactiveResponse>(result.clone()) {
                return Ok(proactive);
            }
        }
    }

    // Try direct parse
    match serde_json::from_str::<ProactiveResponse>(&resp_body) {
        Ok(p) => Ok(p),
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
