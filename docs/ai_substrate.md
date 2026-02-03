# AI Substrate Requirements (AetherOS)

Goal: ensure AetherOS evolves into a *practical substrate* for training + running the world-model stack (CFC-JEPA + ops tooling). This document defines OS-facing interfaces and minimum capabilities so later ports (especially to Aero) are mostly implementation swaps, not redesigns.

## Phase framing
- **Phase 0 (now):** Training/inference runs on Linux, but we standardize interfaces and artifacts.
- **Phase 1:** AetherOS provides the ops daemon + telemetry + job runner (even if it shells out to Linux initially).
- **Phase 2:** Native execution paths expand (IO/scheduling/memory) and eventually GPU runtime.

## Stable interfaces (language-agnostic)
These should remain stable so we can port implementations to Aero later.

### 1) Job Runner
**Responsibilities**
- Start/stop jobs (training, validation, probes)
- Capture stdout/stderr, exit codes, runtime
- Persist job metadata + artifacts

**Minimal API (concept)**
- `POST /jobs` → returns job_id
- `GET /jobs/{id}` → status, runtime, latest metrics
- `POST /jobs/{id}/stop`

### 2) Metrics
**Responsibilities**
- counters/gauges/histograms with timestamps
- export to JSON on disk + optionally an HTTP endpoint

**Schema**
- `metric(name, kind, value, ts, tags)`

### 3) Event Bus / Alerts
**Responsibilities**
- emit structured events (training new best, stalls, sustained dip, gate anomalies)
- apply routing rules (WhatsApp/console/file)

### 4) Artifact Store
**Responsibilities**
- store models, validation reports, plots, PDFs
- consistent paths and retention policy

## OS capabilities (what we need)
### Scheduling / Isolation
- background jobs, priorities, watchdogs
- process groups and kill semantics

### Storage / IO
- fast sequential reads (datasets)
- mmap support and page cache behavior
- async IO primitives

### Observability
- monotonic clock
- structured logging
- trace hooks

### GPU story (later)
- define driver + userland boundary
- resource accounting

## Aero port plan (snappy path)
Order of porting to Aero (highest leverage first):
1) metric/event serialization + parsers
2) artifact indexer + file server for dashboard
3) job runner orchestration (shell-out)
4) native job execution primitives
