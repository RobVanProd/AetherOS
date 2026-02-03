# Build Toolchain (AetherOS)

This repo aims to be **source-only**. Large binary artifacts (e.g. `myos.bin`, `nasm.zip`) are not kept in git.

## Requirements

### For legacy/MyOS build + QEMU boot
- `make`
- `nasm`
- `qemu-system-i386` (or `qemu-system-x86` depending on distro)
- `i686-elf-gcc` cross-compiler toolchain (recommended for OSDev)

## Notes
- If you need prebuilt artifacts for convenience, they should live in **GitHub Releases**.
- For now, build via:
  - `./tools/run_qemu.sh`

## TODO
- Add a reproducible toolchain installer script (Linux) and a pinned version list.
