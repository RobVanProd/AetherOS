# v0 OS Demo Stub (AetherOS)

This is a **demoable proof-of-concept slice** for the Aurora boundary.
It provides two local Unix-socket HTTP/JSON services:
- `aetherd` — substrate stub (`GET /v0/health`)
- `aurorad` — orchestrator stub (`GET /v0/health`, `POST /v0/jobs` mocked)

## Prereqs
- Rust toolchain (`rustc`, `cargo`)

## Run
From repo root:

```bash
cd forge
cargo run -p aetherd
```

In another terminal:

```bash
cd forge
cargo run -p aurorad
```

## Test the endpoints
Health:

```bash
curl --unix-socket /tmp/aetherd.sock http://localhost/v0/health
curl --unix-socket /tmp/aurorad.sock http://localhost/v0/health
```

Submit a mocked job:

```bash
curl --unix-socket /tmp/aurorad.sock \
  -H 'Content-Type: application/json' \
  -d '{"job_type":"predict_next_state"}' \
  http://localhost/v0/jobs
```

## What this proves
- The **integration boundary is real and runnable** (IPC + schema shape)
- We can plug in Aurora CFC behind `POST /v0/jobs` without changing OS clients
