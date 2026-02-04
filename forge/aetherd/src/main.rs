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

fn handle_conn(mut stream: UnixStream) -> anyhow::Result<()> {
    let mut buf = [0u8; 8192];
    let n = stream.read(&mut buf)?;
    let req = String::from_utf8_lossy(&buf[..n]);

    // Very small HTTP parser: only needs method + path for v0 demo.
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

    // default
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
