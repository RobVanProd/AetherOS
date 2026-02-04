# AetherOS v0 Definition — “GenAI Built-In OS”

This document defines what we mean by *the first generative‑AI built‑in OS* in practical, engineering terms.

## North Star
AetherOS is an OS where **intent → action** is a first‑class interface:
- You express what you want (natural language / structured intent)
- The OS composes tools (“facets”) to do it
- Context persists and is queryable
- The system is observable, auditable, and safe by default

## v0 (Prototype) — Definition of Done
v0 is not “AGI in the kernel”. v0 is **a bootable system** where the AI layer is *tightly integrated* with OS primitives.

### Must-haves (v0)
1) **Boot + Interactive shell**
   - Boots reliably in QEMU and at least one real machine
   - Has an interactive console (TUI ok)

2) **Job runner + artifact store (local)**
   - Run “jobs” (builds, evals, small training/inference tasks)
   - Capture logs + exit codes
   - Store artifacts in a consistent layout

3) **Memory / retrieval primitive**
   - OS-owned local memory store for user + system notes
   - Search/recall API with citations (file/line) and clear provenance

4) **Intent interface (Nebula v0)**
   - A single omni‑bar / command surface
   - Can route intents to:
     - shell commands
     - file lookup
     - job runner operations
   - Minimal permissions model (deny-by-default)

5) **Observability**
   - Structured logs
   - Metrics + health status
   - Crash-safe journaling of key events

### Nice-to-haves (v0)
- Networking (DHCP + basic tools)
- Persistent storage (install-to-disk)
- SSH access

## v1+ (post-v0)
- Package management
- Graphics stack (framebuffer → DRM/KMS → compositor as needed)
- GPU runtime integration for local models
- Rich facet marketplace + sandboxing

## Guiding Principles
- **Ship small, demoable slices** (weekly wins)
- **Source-only repo** (binaries in Releases)
- **Test harness first** (QEMU + reproducible builds)
- **Interfaces before implementations** (AI substrate stays stable; can swap language/runtime later)
