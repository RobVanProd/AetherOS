# AetherOS

AetherOS has Makefile targets for serial and graphical QEMU boots, five Cargo manifests under `forge/`, a QEMU launch script, and a vendored legacy `MyOS` tree.

## What It Is

AetherOS is an operating-system prototype exploring an intent-facing shell and AI-adjacent system services. The repo brings together a Linux/QEMU boot harness, Rust daemon crates, a framebuffer shell, a TUI shell, a Python brain server, initramfs tooling, and earlier OS work under `legacy/MyOS`.

The most interesting thing here is the integration target: an OS-shaped environment where shell, dashboard, job routing, audit, and model-backed command processing are treated as parts of the same system.

## Current Status

The repo includes build targets such as `make build`, `make boot`, `make boot-gui`, `make brain-demo`, and `make brain-demo-gui`. `PROGRESS.md` documents serial and graphical boot paths, a framebuffer UI, audio work, mouse cursor work, and known runtime-verification gaps.

No QEMU boot was run for this README. Treat the current state as a research prototype with documented build paths, not a verified operating-system release.

## Tech Stack

- Rust daemons and shells
- Python brain server
- QEMU boot harness
- BusyBox/initramfs tooling
- Linux kernel build scripts
- `ratatui`, `crossterm`, `tiny-skia`, `fontdue`, and related shell/UI crates

## Limitations

The repo contains status docs with ambitious claims, but this README only claims what is visible from files and commands. Audio is explicitly marked as needing runtime verification in `PROGRESS.md`. Automated boot CI is listed as future work.
