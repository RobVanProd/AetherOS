# Aurora ↔ CFC Integration Boundary Proposal (v0)

This document proposes a **clean boundary** for integrating **Aurora** (agent loop / orchestration) with the **CFC stack** (e.g., CFC-JEPA + ops tooling) inside AetherOS.

**Goal:** make Aurora and the CFC runtime *replaceable implementation details* behind stable OS-facing interfaces.

**Non-goals (v0):**
- Running a giant model in kernel space
- Designing the final seL4/Aero security model
- Solving multi-tenant security for untrusted third-party code

---

## 1) Architectural stance

### Principle: "AI lives in userspace"
- The **kernel** exposes primitives (proc/mem/fs/net) and enforces isolation.
- A small set of **system daemons** provide stable OS services.
- Aurora runs as a **supervised userspace service** (daemon) that talks to OS services via IPC.

### Split the problem into 3 layers
1) **Aurora Orchestrator** (planning + tool routing)
2) **CFC Runtime** (training/inference execution, model IO)
3) **Aether Substrate Services** (jobs, artifacts, memory, events, metrics)

Aurora may call the CFC runtime *through the same substrate*, instead of embedding bespoke logic.

---

## 2) Proposed service topology (v0)

### 2.1 Core daemons
- `aetherd` (system substrate daemon)
  - Owns: job runner, artifact index, memory store API, event bus, metrics export
  - Runs as root or a privileged system user (minimal caps)

- `aurorad` (Aurora orchestrator)
  - Owns: intent parsing, planning, policy checks, tool routing
  - Runs unprivileged; talks to `aetherd` over local IPC

- `cfcd` (CFC runtime service) — optional in v0
  - Owns: model runtime (inference + training entrypoints)
  - Can be a thin wrapper around existing Linux tooling initially (Phase 0/1)

### 2.2 Why a separate `cfcd`?
Keeps Aurora smaller and lets you swap runtime implementations:
- local CUDA runtime vs remote inference
- different world-model stacks
- different model formats

If v0 is too early, **collapse `cfcd` into job runner jobs** and revisit later.

---

## 3) IPC boundaries + API sketch

### 3.1 Transport
v0 recommendation:
- **Unix domain socket** + **HTTP/JSON** (simple, debuggable)
  - socket: `/run/aether/aetherd.sock`
  - permissions: `root:aether` group; `0660`

Future:
- gRPC over unix socket (typed, streaming)
- capnproto / flatbuffers for performance

### 3.2 AuthN/AuthZ
- **Local auth** via unix socket permissions (group membership)
- Per-request **capability token** (optional) for audit + least privilege
- **Policy** is enforced centrally in `aetherd` (deny-by-default)

### 3.3 Minimal API (v0)
A stable substrate API, shared by Aurora and other tools.

**Jobs**
- `POST /v0/jobs` → `{ job_id }`
- `GET /v0/jobs/{job_id}` → status/exit/runtime
- `GET /v0/jobs/{job_id}/logs?follow=1` → stream JSONL/logs
- `POST /v0/jobs/{job_id}/stop` → graceful
- `POST /v0/jobs/{job_id}/kill` → immediate

**Artifacts**
- `POST /v0/artifacts` → register artifact path + metadata (type, producing job, hashes)
- `GET /v0/artifacts/{id}` → metadata + filesystem path
- `GET /v0/artifacts?job_id=…` → list

**Memory**
- `POST /v0/memory/entries` → add note/document; returns id
- `GET /v0/memory/search?q=…` → results with **provenance/citations**

**Events/Metrics**
- `POST /v0/events` → emit structured event
- `GET /v0/metrics` → snapshot (JSON)

**Intents (optional, if Aurora is not the only client)**
- `POST /v0/intents` → submit intent; returns intent_id; links to jobs/artifacts

---

## 4) Resource limits & sandboxing (v0)

### 4.1 Threat model (v0)
Assume Aurora and CFC components are *trusted but fallible*.
We want:
- blast-radius reduction (bugs, runaway loops)
- predictable system behavior (no host lockups)
- clear audit trail

### 4.2 Sandboxing strategy
**Aurora orchestrator (`aurorad`)**
- Run as unprivileged user: `aurora:aurora`
- Filesystem access:
  - read-only to `/usr`, `/etc`
  - read/write only to:
    - `/var/lib/aether/aurora/`
    - `/var/lib/aether/memory/` (via API preferred)
- Network:
  - default **off**
  - allowlist domains when explicitly granted (v0 can be coarse: on/off)

**Job execution (per-job sandbox)**
- Use **cgroups v2** limits per job:
  - `CPUQuota`
  - `MemoryMax`
  - `PIDsMax`
  - `IOReadBandwidthMax` / `IOWriteBandwidthMax` (if supported)
- Use **namespaces** (where available):
  - mount namespace (chroot-like rootfs)
  - PID namespace
  - network namespace (default none)
- Use **seccomp** profile for high-risk jobs (optional v0)
- Consider **Landlock** for filesystem restrictions (Linux)

**CFC runtime (`cfcd`)**
- Same principles as jobs; treat as a high-resource service
- GPU access (later): mediated through a runtime wrapper; start with coarse gating

### 4.3 Default limits (suggested starting point)
- Aurora daemon:
  - CPU: 1 core
  - RAM: 1–2 GB
  - No network by default

- CFC jobs:
  - CPU: configurable (default 50% of cores)
  - RAM: configurable (default 50% of system)
  - Disk: max artifact size per job

---

## 5) Filesystem layout (convention)

- Runtime sockets:
  - `/run/aether/aetherd.sock`
  - `/run/aether/aurorad.sock` (if needed)

- Persistent state:
  - `/var/lib/aether/jobs/<job_id>/...`
  - `/var/lib/aether/artifacts/...`
  - `/var/lib/aether/memory/...`
  - `/var/lib/aether/aurora/...`

- Logs:
  - `/var/log/aether/aetherd.jsonl`
  - `/var/log/aether/aurorad.jsonl`

---

## 6) Observability & audit

Minimum required fields for every action record:
- `ts` (monotonic + wall)
- `actor` (aurora, user, system)
- `intent_id` (if applicable)
- `job_id` (if applicable)
- `action` (start_job, read_file, write_artifact, network_fetch, etc.)
- `decision` (allowed/denied + policy reason)
- `provenance` (input refs, citations)

---

## 7) Implementation plan (lightweight)

### Phase 0: Documented interface, Linux implementation
1) Implement `aetherd` API endpoints as a small service (language flexible)
2) Add job runner that shells out to host tools (still captured + sandboxed)
3) Add artifact + memory storage on disk

### Phase 1: Aurora uses only the substrate
1) Aurora routes actions via `aetherd` rather than direct shell/files
2) Add deny-by-default permission prompts at the boundary
3) Add structured audit log + correlation ids

### Phase 2: Extract/introduce `cfcd`
1) Move CFC runtime calls into `cfcd` or job templates
2) Add stronger resource accounting and GPU policy

---

## Related
- `docs/ai_substrate.md` (stable interfaces)
- `docs/V0_DEFINITION.md` (v0 must-haves)
- `docs/aether_os/the_forge_architecture.md` (pipeline context)
