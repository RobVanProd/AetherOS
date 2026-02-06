# AetherOS Context

## Overview
AetherOS is the canonical OS project for Aeternum Labs — "the first generative AI built-in OS."
It includes the Nebula shell interface, The Forge build system, and will integrate Aurora CFC.

- **Repo**: https://github.com/RobVanProd/AetherOS
- **Default branch**: `main`
- **Preferred model for OS work**: Claude Opus 4.5

## Current State (as of 2026-02-04)
- Forge demo stubs merged and green (`cargo test` in forge/)
- Forge CI smoke workflow added
- QEMU harness + deps preflight + QUICKSTART merged
- MyOS legacy kernel lives under `legacy/` — to be absorbed into AetherOS
- Local non-git OS project previously at `/home/rob/Aether_OS` (has roadmap/architecture docs + the_forge/nebula dirs)

## Project Structure
- `forge/` - The Forge build system (Rust)
- `legacy/` - Legacy MyOS kernel code
- `nebula/` - Nebula shell (Rust + wgpu) — to be built
- `tools/` - Development tooling
- `docs/` - Documentation including V0 milestone checklist and Aurora CFC integration boundary

## Key Docs
- `docs/V0_MILESTONE_CHECKLIST.md` - v0 milestone definition
- `docs/AURORA_CFC_INTEGRATION_BOUNDARY.md` - How Aurora CFC integrates

## Aurora CFC Integration Plan
- Run Aurora CFC as a local service (daemon)
- gRPC/HTTP + shared-memory option later
- Explicit model versioning + schema
- Resource limits (GPU/CPU quotas)
- OS components consume predictions via a narrow API

## Build & Test
- `make forge-test` - Run Forge tests
- Ensure cargo is on PATH for CI (cron-safe)
