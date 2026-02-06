#!/usr/bin/env python3
"""
Scout Profile → Kernel Config Generator
Converts Scout hardware profile to kernel .config
"""

import json
import sys
from pathlib import Path

# Vendor/Device ID → Kernel Config mapping
DRIVER_MAP = {
    # AMD Storage
    "1022:43f6": ["CONFIG_SATA_AHCI=y", "CONFIG_ATA=y"],
    "1022:7901": ["CONFIG_SATA_AHCI=y", "CONFIG_ATA=y"],

    # Intel Ethernet
    "8086:125c": ["CONFIG_IGB=y", "CONFIG_IGC=y"],  # i225-V
    "8086:15f3": ["CONFIG_E1000E=y"],

    # Aquantia/Marvell Ethernet
    "1d6a:04c0": ["CONFIG_AQTION=y"],
    "1d6a:07b1": ["CONFIG_AQTION=y"],

    # MediaTek WiFi
    "14c3:6639": ["CONFIG_MT76=m", "CONFIG_MT7921E=m"],

    # AMD USB
    "1022:43fd": ["CONFIG_USB_XHCI_HCD=y"],
    "1022:15b6": ["CONFIG_USB_XHCI_HCD=y"],
    "1022:15b7": ["CONFIG_USB_XHCI_HCD=y"],
    "1022:15b8": ["CONFIG_USB_XHCI_HCD=y"],

    # ASMedia USB
    "1b21:2426": ["CONFIG_USB_XHCI_HCD=y"],
    "1b21:2425": ["CONFIG_USB_XHCI_HCD=y"],

    # AMD Radeon
    "1002:744c": ["CONFIG_DRM=y", "CONFIG_DRM_AMDGPU=y"],
    "1002:13c0": ["CONFIG_DRM=y", "CONFIG_DRM_AMDGPU=y"],
}

BASE_CONFIG = """
# Aether OS - Bespoke Kernel Config
# Generated from Scout hardware profile

# Core
CONFIG_64BIT=y
CONFIG_SMP=y
CONFIG_NR_CPUS=32
CONFIG_PRINTK=y
CONFIG_BUG=y

# Essential
CONFIG_TTY=y
CONFIG_SERIAL_8250=y
CONFIG_SERIAL_8250_CONSOLE=y
CONFIG_BLOCK=y
CONFIG_PCI=y
CONFIG_PCIEPORTBUS=y
CONFIG_PCIEAER=y

# Filesystems
CONFIG_EXT4_FS=y
CONFIG_TMPFS=y
CONFIG_PROC_FS=y
CONFIG_SYSFS=y
CONFIG_DEVTMPFS=y
CONFIG_DEVTMPFS_MOUNT=y

# Networking
CONFIG_NET=y
CONFIG_INET=y
CONFIG_PACKET=y
CONFIG_UNIX=y

# Initramfs
CONFIG_BLK_DEV_INITRD=y
CONFIG_RD_GZIP=y

# Binary support
CONFIG_BINFMT_ELF=y
CONFIG_BINFMT_SCRIPT=y

# Disable modules (static kernel)
CONFIG_MODULES=n
"""


def generate_config_from_scout(scout_json: Path) -> str:
    """Generate kernel config from Scout hardware profile"""

    with open(scout_json) as f:
        profile = json.load(f)

    config_options = set()

    # Start with base config
    for line in BASE_CONFIG.strip().split('\n'):
        if line.strip() and not line.startswith('#') and '=' in line:
            config_options.add(line.strip())

    # Map PCI devices to drivers
    print(f"Analyzing {len(profile['pci_devices'])} PCI devices...", file=sys.stderr)

    for device in profile['pci_devices']:
        dev_id = f"{device['vendor_id']}:{device['device_id']}"

        if dev_id in DRIVER_MAP:
            print(f"  {dev_id}: {device['description']}", file=sys.stderr)
            config_options.update(DRIVER_MAP[dev_id])

    # Generate header
    lines = [
        f"# Generated for: {profile.get('hostname', 'unknown')}",
        f"# Machine ID: {profile['machine_id']}",
        f"# CPU: {profile['cpu_model']}",
        f"# Memory: {profile['memory_total_mb']}MB",
        "",
    ]

    lines.extend(sorted(config_options))

    return "\n".join(lines) + "\n"


def main():
    if len(sys.argv) < 2:
        print("Usage: scout_to_config.py <scout_profile.json>", file=sys.stderr)
        sys.exit(1)

    scout_json = Path(sys.argv[1])

    if not scout_json.exists():
        print(f"Error: {scout_json} not found", file=sys.stderr)
        sys.exit(1)

    config = generate_config_from_scout(scout_json)
    print(config)


if __name__ == "__main__":
    main()
