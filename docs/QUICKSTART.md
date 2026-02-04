# AetherOS Quickstart

This repo is the canonical home of AetherOS.

## Current state
Migration is in progress. **The runnable harness currently targets `legacy/MyOS`** while we adopt modules into canonical paths.

## Requirements (Ubuntu/Debian)
```bash
sudo apt-get update
sudo apt-get install -y make nasm qemu-system-x86
# (either `qemu-system-i386` or `qemu-system-x86_64` should end up on PATH)
```

## Run
From the repo root:
```bash
./tools/run_qemu.sh
```

## Next
- See `MIGRATION.md` for the module-by-module plan.
- See `docs/V0_DEFINITION.md` for the practical v0 definition.
