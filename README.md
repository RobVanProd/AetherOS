# AetherOS

**Canonical repo** for Aeternums’ GenAI‑native operating system.

This repo consolidates:
- **MyOS** (legacy kernel + subsystems) → `legacy/MyOS/`
- **Aether_OS** (design docs + tooling) → `docs/aether_os/`

## What we’re building
AetherOS is an OS where **intent is a first‑class interface** (Nebula): you express what you want done, and the OS composes capabilities (“facets”) to do it — with memory, provenance, and safety.

Read the practical definition here:
- `docs/V0_DEFINITION.md`

v0 planning + Aurora boundary docs:
- `docs/V0_MILESTONE_CHECKLIST.md`
- `docs/AURORA_CFC_INTEGRATION_BOUNDARY.md`

## Current status
- Canonical repo is live; migration is in progress.
- See:
  - `MIGRATION.md` (module-by-module plan)
  - `docs/aether_os/STATUS.md` (latest development status)
  - `docs/aether_os/aether_roadmap.md` (phased roadmap)

## Quickstart (today)
Right now the build harness targets legacy/MyOS while we migrate modules.

```bash
# from repo root
./tools/run_qemu.sh
```

### Forge smoke test (Rust)
To run the lightweight Forge CI/smoke check locally (requires the Rust toolchain / `cargo`; see `docs/build_toolchain.md`):

```bash
# from repo root
make forge-test

# (equivalent)
./tools/forge_smoke.sh
# or: cd forge && cargo test --locked
```

Toolchain notes:
- `docs/build_toolchain.md`

## Repo layout
- `legacy/` — imported projects with preserved history
- `docs/` — canonical docs (including Aether_OS imports)
- `tools/` — build/migration scripts

## Contributing
- `CONTRIBUTING.md`

---

**Principle:** ship small, demoable slices; keep the repo source-only; make the build/test harness boring and reproducible.
