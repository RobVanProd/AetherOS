# Inventory: Aether_OS ↔ MyOS overlap

## Imported sources
- `docs/aether_os/*` — design/roadmap/session notes from local Aether_OS
- `legacy/MyOS/*` — full MyOS tree imported via git subtree

## Next: what to inventory
1) Build system differences
   - toolchain expectations, QEMU scripts, cross compiler, Makefile targets
2) Kernel architecture docs
   - compare Aether roadmaps vs MyOS implemented modules
3) Shared concepts to unify
   - scheduler/process model
   - memory model / paging
   - driver model
   - GUI/windowing/shell apps

## Proposed merge order (module-by-module)
1) Build + run harness (repro build, QEMU boot)
2) Memory/paging + heap allocator
3) Process + scheduler
4) FS
5) Networking/driver(s)
6) Graphics/windowing
7) Shell + apps

## Immediate cleanup candidates (MyOS)
- Remove binaries from version control (e.g., myos.bin, zips) — keep in Releases.
- Add CI that at least builds + boots.
