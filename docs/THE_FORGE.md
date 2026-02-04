# The Forge (Cartographer → Architect → Foundry → Crucible)

The Forge is AetherOS’s **pipeline** for generating many synthetic machine configs, building kernels, and boot-testing them under QEMU.

This is the canonical way to reproduce the “100+ machine / 100% boot pass” validation described in `docs/aether_os/SESSION_SUMMARY.md`.

## Quick start (Docker)

From the repo root:

```bash
cd forge
docker compose build
docker compose up forge
```

Outputs are written via bind-mounts:
- `forge/machines/` — generated machine configs
- `forge/images/` — built kernel/initramfs artifacts (generated)
- `forge/results/` — Crucible boot test results (generated)

## Run individual stages

```bash
cd forge

# extract driver manifest
docker compose run --rm cartographer

# generate machine configs
docker compose run --rm architect

# build kernel/images
docker compose run --rm foundry

# boot-test many machines under QEMU
docker compose run --rm crucible
```

## Notes
- Some hosts need extra permissions for KVM acceleration. The compose file uses `privileged: true` for the `forge` and `crucible` services.
- The Forge container installs QEMU inside the image (`qemu-system-x86`).
