# Changelog

## [1.0.0] - 2026-01-03

### Added
- Initial release of Aether OS
- Minimal bootable Linux kernel (6.6.70)
- Automated kernel generation pipeline (The Forge)
- Multi-machine testing framework
- Support for 8 different machine configurations (minimal and desktop profiles)
- BusyBox-based initramfs with essential utilities
- Serial console output for debugging
- Automated boot validation via QEMU

### Components
- **Cartographer**: Driver manifest extraction from kernel source
- **Architect**: Synthetic machine configuration generation
- **Foundry**: Minimal kernel compilation with optimized config
- **Crucible**: Automated boot testing framework

### Technical Details
- Kernel version: 6.6.70
- Architecture: x86_64
- Boot method: Direct kernel boot with initramfs
- Init system: Minimal shell-based init
- File systems: ext4, tmpfs, proc, sysfs, devtmpfs
- Virtualization: QEMU with KVM support
- Test success rate: 100% (8/8 machines)

### Fixed
- CONFIG_BINFMT_SCRIPT support for shell script execution
- CONFIG_BINFMT_ELF support for binary execution
- Initramfs init script execution
- BusyBox symlink configuration

### Known Limitations
- No network support in current build
- No module loading (monolithic kernel)
- Minimal userspace (BusyBox only)
- No persistent storage
- Auto-shutdown after 5 seconds
