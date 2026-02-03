# Migration Plan (AetherOS)

## Goals
- Preserve history of existing projects.
- Avoid “big bang” merges.
- Import docs/tooling first, then port code module-by-module.

## Step 0 — Snapshot + decisions
- Canonical repo: **AetherOS** (this repo)
- Keep existing repos intact (MyOS remains as-is).

## Step 1 — Import MyOS with history
Preferred: `git subtree` (keeps history, cleanly nests):

```bash
git remote add myos https://github.com/RobVanProd/MyOS.git
git fetch myos
git subtree add --prefix legacy/MyOS myos master --squash=false
```

If `subtree` is unavailable, fallback is `git filter-repo` or a manual history-preserving merge.

## Step 2 — Import Aether_OS docs/tooling (local)
- Copy docs into `docs/aether_os/` with provenance.
- Only port tooling that is used (avoid carrying dead scripts).

Proposed copy targets:
- `aether_roadmap.md`
- `the_forge_architecture.md`
- `SESSION_SUMMARY.md`, `STATUS.md`
- `the_forge/`, `nebula/` docs (if any)

## Step 3 — Inventory overlap + merge order
Create `docs/inventory.md` capturing:
- overlapping subsystems
- maturity level (implemented vs design)
- migration priority

Suggested order:
1) Build/repro tooling + QEMU harness
2) Memory/paging + allocator
3) Process/scheduler
4) FS
5) Drivers (rtl8139)
6) Graphics/windowing
7) Shell/apps

## Step 4 — “Adopt” modules into canonical tree
As modules stabilize, move from `legacy/MyOS/...` into canonical paths:
- `kernel/`
- `drivers/`
- `userspace/`

Each move includes:
- build passes
- a short design note
- a demo/smoke test

## Step 5 — Publishing
Once structure is stable, create a GitHub repo and push.
(Ask before doing any external push.)
