#!/usr/bin/env python3
"""
Kernel Config Generator
Reads machine JSON and generates a bespoke .config with only required drivers.
"""

import json
import sys
from pathlib import Path

# Hardware → Kernel config mapping
HARDWARE_CONFIG_MAP = {
    "storage": {
        "virtio-blk": ["CONFIG_VIRTIO_BLK=y", "CONFIG_VIRTIO=y", "CONFIG_VIRTIO_PCI=y"],
        "ide-hd": ["CONFIG_ATA=y", "CONFIG_ATA_PIIX=y", "CONFIG_BLK_DEV_SD=y"],
        "ide-cd": ["CONFIG_ATA=y", "CONFIG_ATA_PIIX=y", "CONFIG_BLK_DEV_SR=y"],
        "nvme": ["CONFIG_BLK_DEV_NVME=y", "CONFIG_NVME_CORE=y"],
        "scsi-hd": ["CONFIG_SCSI=y", "CONFIG_BLK_DEV_SD=y", "CONFIG_SCSI_VIRTIO=y", "CONFIG_VIRTIO=y", "CONFIG_VIRTIO_PCI=y"],
        "usb-storage": ["CONFIG_USB_STORAGE=y", "CONFIG_USB=y", "CONFIG_SCSI=y", "CONFIG_BLK_DEV_SD=y"],
    },
    "network": {
        "virtio-net-pci": ["CONFIG_VIRTIO_NET=y", "CONFIG_VIRTIO=y", "CONFIG_VIRTIO_PCI=y"],
        "e1000": ["CONFIG_E1000=y"],
        "e1000e": ["CONFIG_E1000E=y"],
        "rtl8139": ["CONFIG_8139CP=y", "CONFIG_8139TOO=y"],
        "ne2k_pci": ["CONFIG_NE2K_PCI=y"],
        "pcnet": ["CONFIG_PCNET32=y"],
        "vmxnet3": ["CONFIG_VMXNET3=y"],
    },
    "usb_controller": {
        "piix3-usb-uhci": ["CONFIG_USB=y", "CONFIG_USB_UHCI_HCD=y"],
        "usb-ehci": ["CONFIG_USB=y", "CONFIG_USB_EHCI_HCD=y"],
        "qemu-xhci": ["CONFIG_USB=y", "CONFIG_USB_XHCI_HCD=y"],
    },
    "usb_devices": {
        "usb-kbd": ["CONFIG_USB_HID=y", "CONFIG_HID=y", "CONFIG_HID_GENERIC=y"],
        "usb-mouse": ["CONFIG_USB_HID=y", "CONFIG_HID=y", "CONFIG_HID_GENERIC=y"],
        "usb-tablet": ["CONFIG_USB_HID=y", "CONFIG_HID=y", "CONFIG_HID_GENERIC=y", "CONFIG_INPUT_EVDEV=y"],
    },
    "gpu": {
        "std": ["CONFIG_DRM=y", "CONFIG_DRM_BOCHS=y"],
        "cirrus": ["CONFIG_DRM=y", "CONFIG_DRM_CIRRUS_QEMU=y"],
        "qxl": ["CONFIG_DRM=y", "CONFIG_DRM_QXL=y"],
        "virtio-vga": ["CONFIG_DRM=y", "CONFIG_DRM_VIRTIO_GPU=y", "CONFIG_VIRTIO=y", "CONFIG_VIRTIO_PCI=y"],
        "vmware-svga": ["CONFIG_DRM=y", "CONFIG_DRM_VMWGFX=y"],
    },
    "sound": {
        "ac97": ["CONFIG_SOUND=y", "CONFIG_SND=y", "CONFIG_SND_AC97_CODEC=y"],
        "es1370": ["CONFIG_SOUND=y", "CONFIG_SND=y", "CONFIG_SND_ENS1370=y"],
        "intel-hda": ["CONFIG_SOUND=y", "CONFIG_SND=y", "CONFIG_SND_HDA_INTEL=y"],
        "sb16": ["CONFIG_SOUND=y", "CONFIG_SND=y", "CONFIG_SND_SB16=y"],
    },
}

# Base config required for all machines
BASE_CONFIG = """
# Core
CONFIG_64BIT=y
CONFIG_SMP=y
CONFIG_PRINTK=y
CONFIG_BUG=y
CONFIG_FUTEX=y
CONFIG_EPOLL=y
CONFIG_SIGNALFD=y
CONFIG_TIMERFD=y
CONFIG_EVENTFD=y

# TTY/Console (for serial output)
CONFIG_TTY=y
CONFIG_SERIAL_8250=y
CONFIG_SERIAL_8250_CONSOLE=y
CONFIG_CONSOLE_TRANSLATIONS=y
CONFIG_VT=n

# Block layer
CONFIG_BLOCK=y
CONFIG_BLK_DEV=y

# File systems (minimal)
CONFIG_EXT4_FS=y
CONFIG_TMPFS=y
CONFIG_PROC_FS=y
CONFIG_SYSFS=y
CONFIG_DEVTMPFS=y
CONFIG_DEVTMPFS_MOUNT=y

# PCI
CONFIG_PCI=y

# Initramfs
CONFIG_BLK_DEV_INITRD=y
CONFIG_RD_GZIP=y

# Binary format support
CONFIG_BINFMT_ELF=y
CONFIG_BINFMT_SCRIPT=y

# Networking stack (if any network hardware)
CONFIG_NET=y
CONFIG_INET=y
CONFIG_PACKET=y
CONFIG_UNIX=y

# Disable modules (static kernel)
CONFIG_MODULES=n

# Disable debug
CONFIG_DEBUG_KERNEL=n
CONFIG_DEBUG_INFO=n
"""


def generate_config_for_machine(machine_json_path: Path) -> str:
    """Generate kernel .config for a specific machine."""

    with open(machine_json_path) as f:
        machine = json.load(f)

    config_options = set()

    # Start with base config
    for line in BASE_CONFIG.strip().split('\n'):
        if line.strip() and not line.startswith('#'):
            config_options.add(line.strip())

    # Storage
    if storage := machine.get("storage"):
        storage_type = storage.get("type")
        if storage_type in HARDWARE_CONFIG_MAP["storage"]:
            config_options.update(HARDWARE_CONFIG_MAP["storage"][storage_type])

    # Network
    if network := machine.get("network"):
        network_type = network.get("type")
        if network_type in HARDWARE_CONFIG_MAP["network"]:
            config_options.update(HARDWARE_CONFIG_MAP["network"][network_type])

    # USB controller
    if usb_controller := machine.get("usb_controller"):
        if usb_controller in HARDWARE_CONFIG_MAP["usb_controller"]:
            config_options.update(HARDWARE_CONFIG_MAP["usb_controller"][usb_controller])

    # USB devices
    if usb_devices := machine.get("usb_devices"):
        for device in usb_devices:
            device_type = device.get("type")
            if device_type in HARDWARE_CONFIG_MAP["usb_devices"]:
                config_options.update(HARDWARE_CONFIG_MAP["usb_devices"][device_type])

    # GPU
    if gpu := machine.get("gpu"):
        gpu_model = gpu.get("model")
        if gpu_model in HARDWARE_CONFIG_MAP["gpu"]:
            config_options.update(HARDWARE_CONFIG_MAP["gpu"][gpu_model])

    # Sound
    if sound := machine.get("sound"):
        sound_model = sound.get("model")
        if sound_model in HARDWARE_CONFIG_MAP["sound"]:
            config_options.update(HARDWARE_CONFIG_MAP["sound"][sound_model])

    # Generate config file
    config_lines = ["# Generated config for machine: " + machine.get("machine_id", "unknown")]
    config_lines.append(f"# Profile: {machine.get('profile', 'unknown')}")
    config_lines.append("")
    config_lines.extend(sorted(config_options))

    return "\n".join(config_lines) + "\n"


def main():
    if len(sys.argv) < 2:
        print("Usage: generate_config.py <machine_json> [output_config]", file=sys.stderr)
        sys.exit(1)

    machine_json = Path(sys.argv[1])
    output_config = Path(sys.argv[2]) if len(sys.argv) > 2 else None

    if not machine_json.exists():
        print(f"Error: Machine JSON not found: {machine_json}", file=sys.stderr)
        sys.exit(1)

    config_content = generate_config_for_machine(machine_json)

    if output_config:
        output_config.write_text(config_content)
        print(f"Config written to: {output_config}")
    else:
        print(config_content)


if __name__ == "__main__":
    main()
