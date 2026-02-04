# Aurora CFC Integration Boundary (AetherOS)

Goal: integrate the Aurora **CFC** world-model capability into AetherOS as a **service** behind a narrow, auditable interface.

This document is intentionally v0-oriented: simple IPC first, harden later.

---

## Service topology (v0 → v1)
### v0 (minimum viable)
- **`aetherd`** (substrate / policy / audit)
  - owns authz decisions, policy checks, and audit log
  - provides a stable socket for clients
- **`aurorad`** (orchestrator)
  - job queue, artifact management, model lifecycle
  - talks to model runtime locally
- Optional (can be in-process inside `aurorad` for v0): **`cfcd`** (model runtime)
  - loads model weights
  - runs inference

### Why split
- Keeps OS-facing API stable while model internals churn.
- Makes sandboxing/resource limits easier.

---

## IPC (v0)
- Transport: **Unix domain socket**
- Protocol: **HTTP/JSON**
  - easy to debug with `curl --unix-socket`
  - easy to replace with gRPC later

Future (v1): gRPC + optional shared-memory fast path.

---

## API sketch
All requests/records include:
- `request_id`, `job_id`
- `model_id`, `model_version`, `schema_version`
- `input_hash` (sha256)
- timestamps + timings

### Endpoints
- `GET /v0/health` → ok + versions
- `POST /v0/jobs` → submit an inference job
- `GET /v0/jobs/{job_id}` → status
- `GET /v0/jobs/{job_id}/result` → result (or artifact reference)
- `GET /v0/artifacts/{id}` → fetch artifact
- `GET /v0/metrics` → basic timings/counters

### Job types (examples)
- `predict_next_state`
- `encode_state`
- `score_sequence`

---

## Filesystem layout
Proposed base: `/var/lib/aether/aurora/`
- `models/` — model packages (versioned)
- `cache/` — ephemeral caches
- `artifacts/` — job outputs (versioned, content-addressed when possible)
- `logs/` — structured logs + audit trail

---

## Sandboxing + resource limits
v0 target: strong defaults without blocking dev.

- **cgroups v2** for CPU/mem limits
- Linux namespaces
  - network namespace **off by default** (no net)
  - mount namespace to restrict filesystem
- Optional hardening (v1):
  - seccomp profile
  - landlock where applicable

---

## Phased implementation plan
1) **Stub services**
- `aetherd` + `aurorad` respond to `health` + accept a fake job

2) **Artifact plumbing**
- job submission → artifact storage → retrieval

3) **Model runtime integration**
- `aurorad` can call a local `cfcd` inference entrypoint

4) **Hardening**
- resource limits on by default
- audit log schema stable

---

## Success criteria (v0)
- OS-side client can submit a job and receive a response.
- Every inference is auditable (who/what/when/model version/timings).
- Model runtime is sandboxed by default (at least “no network” + resource caps).
