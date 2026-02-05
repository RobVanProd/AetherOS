# Build Toolchain (AetherOS)

This repo aims to be **source-only**. Large binary artifacts (e.g. `myos.bin`, `nasm.zip`) are not kept in git.

## Requirements

### For legacy/MyOS build + QEMU boot
- `make`
- `nasm`
- `qemu-system-i386` (or `qemu-system-x86` depending on distro)
- `i686-elf-gcc` cross-compiler toolchain (recommended for OSDev)

### For Forge smoke tests (Rust)
- Rust toolchain (`cargo`)
  - Recommended install via `rustup`: https://rustup.rs
  - Verify: `cargo --version`

## Notes
- If you need prebuilt artifacts for convenience, they should live in **GitHub Releases**.

### Run (current harness)
From the repo root:
```bash
./tools/run_qemu.sh
```

This currently builds/runs **legacy/MyOS** while we migrate modules into canonical paths.

## TODO
- Add a reproducible toolchain installer script (Linux) and a pinned version list.
