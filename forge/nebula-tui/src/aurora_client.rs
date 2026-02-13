use std::io::{Read, Write};
use std::net::TcpStream;
use std::os::unix::net::UnixStream;
use std::time::Duration;

/// Aurora/aurorad connection status.
#[derive(Default, Clone)]
pub struct AuroraStatus {
    pub connected: bool,
    pub predictions: u64,
    pub weight_version: String,
    pub learning_enabled: bool,
    pub error: f64,
    pub gate_stats: Vec<String>,
    pub latency_ms: u64,
}

/// Structured prediction insight from CFC-JEPA.
#[derive(Clone, Debug)]
pub struct PredictionInsight {
    pub prediction_error: f64,
    pub latency_ms: f64,
    pub learning_enabled: bool,
    pub total_predictions: u64,
    pub weight_version: String,
}

/// Introspection data from CFC-JEPA.
#[derive(Clone, Debug)]
pub struct IntrospectData {
    pub weight_version: String,
    pub param_count: u64,
    pub total_predictions: u64,
    pub mean_latency_ms: f64,
    pub learning_enabled: bool,
    pub total_updates: u64,
    pub mean_prediction_error: f64,
}

/// How to reach aurorad.
fn aurorad_addr() -> AuroraAddr {
    if let Ok(port) = std::env::var("AURORAD_TCP_PORT") {
        if let Ok(p) = port.parse::<u16>() {
            return AuroraAddr::Tcp(format!("127.0.0.1:{}", p));
        }
    }
    if let Ok(host) = std::env::var("AURORAD_HOST") {
        return AuroraAddr::Tcp(host);
    }
    let sock = std::env::var("AURORAD_SOCKET")
        .unwrap_or_else(|_| "/tmp/aurorad.sock".to_string());
    AuroraAddr::Unix(sock)
}

enum AuroraAddr {
    Unix(String),
    Tcp(String),
}

fn http_get(addr: &AuroraAddr, path: &str) -> Result<String, String> {
    let request =
        format!("GET {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n");

    match addr {
        AuroraAddr::Unix(sock) => {
            let mut stream = UnixStream::connect(sock).map_err(|e| e.to_string())?;
            stream.set_read_timeout(Some(Duration::from_secs(2))).ok();
            stream
                .write_all(request.as_bytes())
                .map_err(|e| e.to_string())?;
            let mut resp = String::new();
            stream
                .read_to_string(&mut resp)
                .map_err(|e| e.to_string())?;
            extract_body(&resp)
        }
        AuroraAddr::Tcp(host) => {
            let mut stream = TcpStream::connect(host).map_err(|e| e.to_string())?;
            stream.set_read_timeout(Some(Duration::from_secs(2))).ok();
            stream
                .write_all(request.as_bytes())
                .map_err(|e| e.to_string())?;
            let mut resp = String::new();
            stream
                .read_to_string(&mut resp)
                .map_err(|e| e.to_string())?;
            extract_body(&resp)
        }
    }
}

fn http_post(addr: &AuroraAddr, path: &str, body: &str) -> Result<String, String> {
    let request = format!(
        "POST {path} HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );

    match addr {
        AuroraAddr::Unix(sock) => {
            let mut stream = UnixStream::connect(sock).map_err(|e| e.to_string())?;
            stream.set_read_timeout(Some(Duration::from_secs(5))).ok();
            stream
                .write_all(request.as_bytes())
                .map_err(|e| e.to_string())?;
            let mut resp = String::new();
            stream
                .read_to_string(&mut resp)
                .map_err(|e| e.to_string())?;
            extract_body(&resp)
        }
        AuroraAddr::Tcp(host) => {
            let mut stream = TcpStream::connect(host).map_err(|e| e.to_string())?;
            stream.set_read_timeout(Some(Duration::from_secs(5))).ok();
            stream
                .write_all(request.as_bytes())
                .map_err(|e| e.to_string())?;
            let mut resp = String::new();
            stream
                .read_to_string(&mut resp)
                .map_err(|e| e.to_string())?;
            extract_body(&resp)
        }
    }
}

fn extract_body(resp: &str) -> Result<String, String> {
    if let Some(idx) = resp.find("\r\n\r\n") {
        Ok(resp[idx + 4..].to_string())
    } else {
        Ok(resp.to_string())
    }
}

/// Check aurorad health.
pub fn check_health() -> AuroraStatus {
    let addr = aurorad_addr();
    match http_get(&addr, "/v0/health") {
        Ok(body) => {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&body) {
                if v.get("ok").and_then(|o| o.as_bool()) == Some(true) {
                    return AuroraStatus {
                        connected: true,
                        ..Default::default()
                    };
                }
            }
            AuroraStatus::default()
        }
        Err(_) => AuroraStatus::default(),
    }
}

/// Query predict endpoint and parse structured response.
pub fn query_prediction() -> Result<PredictionInsight, String> {
    let addr = aurorad_addr();
    let features: Vec<f64> = vec![0.5; 128]; // placeholder features
    let body = serde_json::json!({
        "job_type": "predict_next_state",
        "state_features": features
    });
    let resp = http_post(&addr, "/v0/jobs", &body.to_string())?;
    let v: serde_json::Value = serde_json::from_str(&resp).map_err(|e| e.to_string())?;

    let result = v.get("result").unwrap_or(&v);

    Ok(PredictionInsight {
        prediction_error: result
            .get("prediction_error")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0),
        latency_ms: result
            .get("latency_ms")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0),
        learning_enabled: result
            .get("learning_enabled")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        total_predictions: result
            .get("total_predictions")
            .and_then(|v| v.as_u64())
            .unwrap_or(0),
        weight_version: result
            .get("weight_version")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string(),
    })
}

/// Query introspect endpoint for full model state.
pub fn query_introspect() -> Result<IntrospectData, String> {
    let addr = aurorad_addr();
    let body = serde_json::json!({"job_type": "introspect"});
    let resp = http_post(&addr, "/v0/jobs", &body.to_string())?;
    let v: serde_json::Value = serde_json::from_str(&resp).map_err(|e| e.to_string())?;

    let result = v.get("result").unwrap_or(&v);
    let model = result.get("model").unwrap_or(result);
    let predictions = result.get("predictions").unwrap_or(result);
    let learning = result.get("learning").unwrap_or(result);

    Ok(IntrospectData {
        weight_version: model
            .get("weight_version")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string(),
        param_count: model
            .get("param_count")
            .and_then(|v| v.as_u64())
            .unwrap_or(0),
        total_predictions: predictions
            .get("total_predictions")
            .and_then(|v| v.as_u64())
            .unwrap_or(0),
        mean_latency_ms: predictions
            .get("mean_latency_ms")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0),
        learning_enabled: learning
            .get("learning_enabled")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        total_updates: learning
            .get("total_updates")
            .and_then(|v| v.as_u64())
            .unwrap_or(0),
        mean_prediction_error: learning
            .get("mean_prediction_error")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0),
    })
}

/// Enable/disable learning.
pub fn set_learning(enable: bool) -> String {
    let addr = aurorad_addr();
    let job_type = if enable {
        "enable_learning"
    } else {
        "disable_learning"
    };
    let body = serde_json::json!({"job_type": job_type});
    match http_post(&addr, "/v0/jobs", &body.to_string()) {
        Ok(resp) => resp,
        Err(e) => format!("Error: {}", e),
    }
}

/// Save weights.
pub fn save_weights() -> String {
    let addr = aurorad_addr();
    let body = serde_json::json!({"job_type": "save_weights"});
    match http_post(&addr, "/v0/jobs", &body.to_string()) {
        Ok(resp) => resp,
        Err(e) => format!("Error: {}", e),
    }
}
