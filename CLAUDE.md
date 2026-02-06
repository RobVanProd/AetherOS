# AetherOS

## Project
AetherOS is "the first generative AI built-in OS" — a complete operating system with Aurora CFC world model integration, the Nebula shell interface, and The Forge build system.

## Quick Reference
- **Test Forge**: `make forge-test`
- **QEMU boot**: See tools/ and docs/QUICKSTART
- **Default branch**: `main`

## Structure
- `forge/` - The Forge build system (Cartographer, Foundry, Crucible)
- `legacy/` - Legacy MyOS kernel (being absorbed into AetherOS)
- `nebula/` - Nebula shell interface (Rust + wgpu) — in development
- `tools/` - Development and QEMU harness tooling
- `docs/` - Architecture docs, milestone checklists, integration specs

## Development Rules
1. Test Forge changes with `make forge-test`
2. Ensure cargo is on PATH for CI (cron-safe)
3. Test in QEMU before bare-metal changes
4. Follow the V0 milestone checklist

## Key Docs
- `docs/V0_MILESTONE_CHECKLIST.md` - v0 definition
- `docs/AURORA_CFC_INTEGRATION_BOUNDARY.md` - Aurora integration spec
- `MIGRATION.md` - MyOS migration notes

## Aurora CFC Integration
Aurora runs as a local daemon with gRPC/HTTP API. OS components consume predictions via a narrow, versioned API with resource limits.
