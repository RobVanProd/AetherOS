# AetherOS

## Project
AetherOS is "the first generative AI built-in OS" — a complete operating system with Aurora CFC world model integration, the Nebula shell interface, and The Forge build system.

## Quick Reference
- **Boot**: `make boot` (build + QEMU)
- **Boot with AI**: `make demo` (boot + cfcd on host)
- **Build only**: `make build`
- **Test Forge**: `make forge-test`
- **Default branch**: `main`

## Structure
- `forge/` - The Forge build system (Cartographer, Foundry, Crucible)
  - `forge/aetherd/` - Audit/policy daemon (Rust, TCP/Unix socket)
  - `forge/aurorad/` - Job routing daemon (Rust, TCP/Unix, forwards to cfcd)
  - `forge/cfcd/` - CFC-JEPA model runtime daemon (Python, 37M-param world model)
  - `forge/nebula-tui/` - Nebula TUI shell (Rust, ratatui, crossterm)
- `aether_init/` - Init system (PID 1 shell script, v0.3)
- `the_forge_original/` - Docker-based kernel build pipeline (Linux 6.6.70)
- `tools/` - Build scripts (build_initramfs.sh, run_qemu.sh)
- `legacy/` - Legacy MyOS kernel (archived)
- `docs/` - Architecture docs, milestone checklists, integration specs

## Boot Architecture
```
HOST (ROCm GPU)                  QEMU VM (AetherOS)
  cfcd (PyTorch)                   PID 1: init script
    ↕ TCP:9100                       ├── aetherd  (TCP:9101)
                                     ├── aurorad  (TCP:9102 → cfcd:9100)
                                     └── nebula-tui (Nebula shell)
```

## Development Rules
1. Build with `make build` before testing
2. All Rust binaries cross-compile to x86_64-unknown-linux-musl (static)
3. Test in QEMU with `make boot` before committing
4. Follow the V0 milestone checklist

## Key Docs
- `docs/V0_MILESTONE_CHECKLIST.md` - v0 definition
- `docs/AURORA_CFC_INTEGRATION_BOUNDARY.md` - Aurora integration spec
- `MIGRATION.md` - MyOS migration notes

## Phase History

### Phase 6: Self-Modifying World Model (DONE)
- **cfcd**: CFC-JEPA checkpoint → inference at ~155ms on ROCm GPU
- **Online learning**: observe → predict → compare → update weights
- **Weight versioning**: Atomic saves, manifest tracking, auto-rollback
- **Demos**: `forge/cfcd/demo_e2e.py`, `forge/cfcd/demo_full_stack.sh`

### Phases 7-11: Bootable Prototype (DONE)
- **Phase 7**: QEMU boot infrastructure (Makefile, initramfs, run_qemu.sh)
- **Phase 8**: Static Rust daemons (aetherd/aurorad with TCP fallback)
- **Phase 9**: Nebula TUI shell (ratatui — system dashboard, AI panel, omni-bar)
- **Phase 10**: cfcd TCP bridge (host:9100 → guest aurorad)
- **Phase 11**: End-to-end integration (`make boot` → Nebula shell)
- **Kernel**: Rebuilt Linux 6.6.70 via Docker with full networking (CONFIG_NET/INET/UNIX)
