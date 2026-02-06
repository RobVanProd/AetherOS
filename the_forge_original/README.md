# Aether OS - The Forge

**v1.0.0** - Automated kernel generation and boot testing pipeline

## Overview

The Forge is an automated build system for Aether OS that extracts driver information from the Linux kernel, generates synthetic machine configurations, compiles minimal bootable kernels, and validates them through automated testing.

## Quick Start

```bash
# One-command build and test
./start.sh

# Or use Docker directly
docker build -t aeternum/forge .
docker run --privileged \
    -v $(pwd)/results:/forge/results \
    -v $(pwd)/images:/forge/images \
    -v $(pwd)/machines:/forge/machines \
    -v $(pwd)/data:/forge/data \
    aeternum/forge:latest all

# Run individual components
docker run aeternum/forge:latest cartographer   # Extract driver manifest
docker run aeternum/forge:latest architect      # Generate machine configs
docker run aeternum/forge:latest foundry        # Compile kernel
docker run --privileged aeternum/forge:latest crucible  # Boot testing
```

## Project Structure

```
the_forge/
├── cartographer/     # Linux kernel parser, extracts driver↔device mappings
├── architect/        # Synthetic machine generator
├── foundry/          # Kernel compiler (Linux for now, seL4 later)
├── crucible/         # QEMU boot testing
├── skills/           # Learned transpilation patterns (future)
├── machines/         # Generated machine_identity.json files
├── images/           # Compiled bootable ISOs
└── results/          # Boot test logs and metrics
```

## Status: v1.0.0 Release ✓

All core components are fully functional:

- [x] **Cartographer**: Extracts 268 PCI and 3447 USB device IDs from kernel source
- [x] **Architect**: Generates synthetic machine configurations (minimal and desktop profiles)
- [x] **Foundry**: Compiles minimal bootable Linux kernel (6.6.70) with optimized config
- [x] **Crucible**: Automated QEMU boot testing with validation
- [x] **Pipeline**: Full end-to-end automation with 100% test success rate

### Test Results (Latest Build)
- **8/8 machines boot successfully**
- **100% pass rate**
- Kernel version: 6.6.70
- Architecture: x86_64
- Boot time: ~5 seconds per machine

## Features

### Cartographer
- Scans Linux kernel source code for hardware driver information
- Extracts PCI and USB device ID mappings
- Categorizes drivers (network, storage, GPU, input, etc.)
- Generates comprehensive driver manifest in JSON format

### Architect
- Creates synthetic machine configurations
- Supports multiple profiles (minimal, desktop)
- Randomized hardware specifications (CPU, RAM, storage)
- Outputs QEMU-compatible machine definitions

### Foundry
- Compiles minimal Linux kernel (6.6.70)
- Optimized configuration for QEMU
- Embedded BusyBox-based initramfs
- Supports essential filesystems (ext4, tmpfs, proc, sysfs)
- Serial console output for debugging

### Crucible
- Automated boot testing via QEMU
- Validates kernel boot success
- Captures serial console output
- Generates test reports with success/failure metrics
- Supports KVM acceleration for faster testing

## Requirements

- Docker (with `--privileged` for KVM acceleration)
- ~4GB disk space (kernel source + build artifacts)
- Linux host (tested on Ubuntu 24.04)
- Optional: KVM support for faster boot testing

## Output Files

After running the pipeline, you'll find:

- `images/vmlinuz` - Bootable kernel binary
- `images/initramfs.cpio.gz` - Initial RAM filesystem
- `images/build_info.json` - Build metadata
- `data/driver_manifest.json` - Extracted driver database
- `machines/*.json` - Generated machine configurations
- `results/summary.json` - Test results summary
- `results/*.log` - Individual boot test logs

## Manual Testing

Boot the kernel directly with QEMU:

```bash
qemu-system-x86_64 \
    -kernel images/vmlinuz \
    -initrd images/initramfs.cpio.gz \
    -append "console=ttyS0" \
    -m 256 \
    -nographic \
    -enable-kvm
```

## Development

Enter the development shell:

```bash
docker run -it --privileged \
    -v $(pwd):/forge \
    aeternum/forge:latest shell
```
