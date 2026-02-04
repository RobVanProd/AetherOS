# v0 Milestone Checklist (AetherOS)

This is a lightweight, **demo-driven** checklist for what “v0” means in practice.
It is aligned with `docs/V0_DEFINITION.md`.

## v0 principles
- **Boots reliably** in the reference environment.
- **One happy-path demo** that proves the core idea.
- **Boring, reproducible tooling** (build/run/test).
- **Narrow interfaces** between subsystems (so we can swap/upgrade).

---

## 0) Repo health / baseline
- [ ] Source-only repo posture (no large binaries committed)
- [ ] One canonical “how to run” path works on a fresh machine
- [ ] CI at least checks formatting/lint + builds the reference target(s)

Evidence:
- `README.md` Quickstart works
- CI workflow logs

---

## 1) Boot + shell slice
- [ ] QEMU harness boots to a deterministic state (no manual steps)
- [ ] Serial console usable (log output stable enough to debug regressions)
- [ ] Minimal shell / command dispatcher works

Demo script:
1. `./tools/run_qemu.sh`
2. Show boot banner + version string
3. Run 2–3 basic commands (help, version, ls-like, echo)

Evidence:
- Boot log artifact
- A short screen recording / asciinema

---

## 2) “AI substrate” boundary (v0)
Goal: make AI an *attachable capability* behind a strict boundary.

- [ ] Define the substrate daemon (`aetherd`) responsibilities (auth, policy, audit)
- [ ] Define the model orchestrator (`aurorad`) responsibilities (jobs, artifacts, lifecycle)
- [ ] Define v0 IPC and message schema

Evidence:
- `docs/AURORA_CFC_INTEGRATION_BOUNDARY.md`
- A stubbed service responds to `health` and `version`

---

## 3) Aurora CFC minimal integration demo
- [ ] Model packaged with a single inference entrypoint
- [ ] OS-side client can submit a job + receive a result
- [ ] Logs are structured and auditable (job id, input hash, model version, timings)

Demo script:
1. Start `aetherd` + `aurorad` (stub OK first)
2. Submit a “predict next state” job (toy input OK)
3. Receive result + show audit log entry

Evidence:
- Example request/response transcript in docs
- One end-to-end integration test (even if mocked model)

---

## 4) Safety / resource limits (v0)
- [ ] CPU/memory limits for model runtime
- [ ] Default: **no network** for model runtime
- [ ] Clear filesystem layout for artifacts, logs, cache

Evidence:
- Documented limits + example config
- `ps`/cgroup evidence in demo notes

---

## Definition of Done (v0)
- A new contributor can run the demo in <30 minutes.
- The AI boundary exists, is testable, and is secured by default.
- The repo has a clear “what’s next” list for v0.1.
