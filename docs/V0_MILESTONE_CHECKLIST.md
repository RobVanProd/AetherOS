# AetherOS v0 Milestone Checklist (Actionable)

This is the **engineering checklist** for reaching **v0** as defined in `docs/V0_DEFINITION.md`.

- Goal: make v0 *demoable, testable, and repeatable*.
- Non-goal: "AGI in the kernel".

## How to use this doc
For each item, aim to provide:
- **Acceptance criteria** (observable)
- **Evidence artifact** (log, screenshot, file path, or command output)
- **Owner + status** (optional, but recommended)

---

## 0) Release hygiene (v0 gating)
- [ ] **Single-command bring-up**
  - Acceptance: `./tools/run_qemu.sh` boots to a prompt in < 30s on a clean machine.
  - Evidence: boot log captured under `artifacts/boot/` or equivalent.
- [ ] **Reproducible build notes**
  - Acceptance: `docs/QUICKSTART.md` contains exact host deps + steps.
  - Evidence: a second person can follow without tribal knowledge.
- [ ] **Version stamping**
  - Acceptance: build embeds version/commit in banner and `aether --version` (or equivalent).

---

## 1) Boot + interactive shell (v0 must-have)
- [ ] **Boot reliably in QEMU**
  - Acceptance: 10/10 consecutive boots on a reference QEMU config.
- [ ] **Boot on at least one real machine**
  - Acceptance: serial/console login reachable; basic commands work.
- [ ] **Interactive console UX**
  - Acceptance: basic help, clear error messages, sensible prompt, `dmesg` accessible.
- [ ] **Crash/boot diagnostics**
  - Acceptance: panic/oops logs persist (or are captured by harness) with timestamps.

---

## 2) Job runner + local artifact store (v0 must-have)
- [ ] **Job submission**
  - Acceptance: `aether job run <cmd>` (or similar) starts a job and returns `job_id`.
- [ ] **Job lifecycle**
  - Acceptance: query status, stream logs, stop/kill job; exit codes recorded.
- [ ] **Artifact store (local)**
  - Acceptance: consistent layout, e.g.:
    - `/var/lib/aether/jobs/<job_id>/logs.{txt,jsonl}`
    - `/var/lib/aether/jobs/<job_id>/artifacts/`
    - `/var/lib/aether/jobs/<job_id>/meta.json`
- [ ] **Retention policy**
  - Acceptance: configurable max age / max disk usage; safe deletion strategy.

---

## 3) Memory / retrieval primitive (v0 must-have)
- [ ] **OS-owned local memory store**
  - Acceptance: stored on persistent disk when available; read-only mode on live media.
- [ ] **Write API**
  - Acceptance: add note with tags + source; returns id.
- [ ] **Search/recall API**
  - Acceptance: keyword + semantic (optional) search returning **citations/provenance**.
  - Evidence: output includes `file:line` or `artifact://…` style pointers.
- [ ] **Data model + migration**
  - Acceptance: versioned schema; migrations are explicit and reversible.

---

## 4) Intent interface (Nebula v0) (v0 must-have)
- [ ] **Single command surface**
  - Acceptance: one UI surface (TUI is fine) accepts freeform intent.
- [ ] **Routing to core facets**
  - Acceptance: at minimum can route to:
    - shell command execution (via job runner)
    - file lookup / memory recall
    - job runner operations (list/status/stop)
- [ ] **Deny-by-default permissions**
  - Acceptance: intents that touch network / disk writes / privileged actions require explicit grant.
- [ ] **Audit trail**
  - Acceptance: every intent produces a structured record: input → plan → actions → artifacts.

---

## 5) Observability (v0 must-have)
- [ ] **Structured logs**
  - Acceptance: JSONL (or similar) logs for core services; include timestamps + correlation id.
- [ ] **Metrics**
  - Acceptance: at least counters for jobs started/failed, boot success, memory store ops.
- [ ] **Health/status**
  - Acceptance: `aether status` (or similar) prints service health, disk usage, last errors.
- [ ] **Crash-safe journaling of key events**
  - Acceptance: intent/job events survive reboots (when persistent storage exists).

---

## Nice-to-haves (v0)
- [ ] **Networking** (DHCP + ping + wget)
- [ ] **Persistent install-to-disk**
- [ ] **SSH access**

---

## v0 demo scripts (recommended)
These are "show me" flows that reduce debate.

- [ ] **Demo A: intent → job → artifact**
  1) Boot
  2) Submit intent: "download example.com and summarize"
  3) Job runs, logs captured
  4) Artifact stored + cited in response

- [ ] **Demo B: memory recall with provenance**
  1) Store a note
  2) Ask an intent that recalls it
  3) Output shows citations/provenance

- [ ] **Demo C: denial & permission grant**
  1) Intent requests network access
  2) System denies until explicit grant
  3) Grant recorded in audit trail
