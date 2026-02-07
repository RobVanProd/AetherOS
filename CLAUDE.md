# AetherOS

## Project
AetherOS is "the first generative AI built-in OS" — a complete operating system with Aurora CFC world model integration, the Nebula shell interface, and The Forge build system.

## Quick Reference
- **Boot**: `make boot` (build + QEMU)
- **Boot with AI**: `make demo` (boot + cfcd on host)
- **Boot with Brain**: `make brain-demo` (brain server + cfcd + QEMU)
- **Build only**: `make build`
- **Test Forge**: `make forge-test`
- **Default branch**: `main`

## Structure
- `forge/` - The Forge build system (Cartographer, Foundry, Crucible)
  - `forge/aetherd/` - Audit/policy daemon (Rust, TCP/Unix socket)
  - `forge/aurorad/` - Job routing daemon (Rust, TCP/Unix, forwards to cfcd + brain)
  - `forge/brain/` - Brain server (Python, Claude API, NL processing + tools)
  - `forge/cfcd/` - CFC-JEPA model runtime daemon (Python, 37M-param world model)
  - `forge/nebula-tui/` - Nebula TUI shell (Rust, ratatui, AI-native omni-bar)
- `aether_init/` - Init system (PID 1 shell script, v0.3)
- `the_forge_original/` - Docker-based kernel build pipeline (Linux 6.6.70)
- `tools/` - Build scripts (build_initramfs.sh, run_qemu.sh)
- `legacy/` - Legacy MyOS kernel (archived)
- `docs/` - Architecture docs, milestone checklists, integration specs

## Boot Architecture
```
HOST (ROCm GPU)                  QEMU VM (AetherOS)
  brain_server (Claude API)        PID 1: init script
    ↕ TCP:9200                       ├── aetherd  (TCP:9101)
  cfcd (PyTorch)                     ├── aurorad  (TCP:9102 → brain:9200, cfcd:9100)
    ↕ TCP:9100                       └── nebula-tui (AI-native omni-bar)
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

### Phase 12: AI-Native OS (DONE)
- **brain_server.py**: Claude-powered NL processing on host (TCP:9200)
  - Tool-use: weather, file ops, search, system info, web fetch, shell commands
  - Uses Claude CLI for auth (OAuth), Sonnet model, ~3-15s latency
  - Structured JSON responses with inline widgets
- **aurorad brain routing**: `job_type: "brain"` forwards to brain_server
- **Nebula TUI overhaul**: Full-width output, OutputBlock enum (Text/Styled/Widget/Separator)
  - Default-to-brain: all input goes to Claude, `!cmd` for shell passthrough
  - Async brain calls via mpsc channel + threads (non-blocking)
  - Inline widget rendering: weather, system, file, table, info
  - "Thinking..." animation while waiting for brain
- **Boot**: `make brain-demo` = brain_server + cfcd + QEMU
