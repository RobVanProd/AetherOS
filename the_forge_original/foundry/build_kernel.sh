#!/bin/bash
#
# The Foundry
# Compiles a minimal bootable Linux kernel for testing.
#
set -e

KERNEL_SRC="/forge/kernel_src"
BUILD_DIR="/forge/build"
IMAGES_DIR="/forge/images"
MACHINES_DIR="/forge/machines"

echo "The Foundry: Building bootable kernel"

# Create directories
mkdir -p "$BUILD_DIR" "$IMAGES_DIR"

cd "$KERNEL_SRC"

# Use a minimal config optimized for QEMU
echo "Generating minimal kernel config..."

# Start with a minimal config
make O="$BUILD_DIR" allnoconfig

# Enable required options via script
cat >> "$BUILD_DIR/.config" << 'EOF'
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

# TTY/Console (for serial + framebuffer output)
CONFIG_TTY=y
CONFIG_SERIAL_8250=y
CONFIG_SERIAL_8250_CONSOLE=y
CONFIG_CONSOLE_TRANSLATIONS=y
CONFIG_VT=y
CONFIG_VT_CONSOLE=y
CONFIG_FRAMEBUFFER_CONSOLE=y

# Block devices
CONFIG_BLOCK=y
CONFIG_BLK_DEV=y
CONFIG_VIRTIO_BLK=y
CONFIG_BLK_DEV_NVME=y
CONFIG_ATA=y
CONFIG_ATA_PIIX=y

# File systems (minimal)
CONFIG_EXT4_FS=y
CONFIG_TMPFS=y
CONFIG_PROC_FS=y
CONFIG_SYSFS=y
CONFIG_DEVTMPFS=y
CONFIG_DEVTMPFS_MOUNT=y

# Virtio (for QEMU)
CONFIG_VIRTIO=y
CONFIG_VIRTIO_PCI=y
CONFIG_VIRTIO_MMIO=y
CONFIG_VIRTIO_CONSOLE=y
CONFIG_HW_RANDOM_VIRTIO=y

# PCI
CONFIG_PCI=y

# Networking stack
CONFIG_NET=y
CONFIG_INET=y
CONFIG_IP_PNP=y
CONFIG_IP_PNP_DHCP=y
CONFIG_PACKET=y
CONFIG_UNIX=y

# Network drivers (QEMU + common hardware)
CONFIG_NETDEVICES=y
CONFIG_ETHERNET=y
CONFIG_E1000=y
CONFIG_E1000E=y
CONFIG_VIRTIO_NET=y
CONFIG_R8169=y
CONFIG_IGB=y
CONFIG_IGC=y

# Initramfs (we'll embed a minimal one)
CONFIG_BLK_DEV_INITRD=y
CONFIG_RD_GZIP=y

# Binary format support (needed to execute init scripts)
CONFIG_BINFMT_ELF=y
CONFIG_BINFMT_SCRIPT=y

# Graphics / DRM / Framebuffer
CONFIG_DRM=y
CONFIG_DRM_KMS_HELPER=y
CONFIG_DRM_BOCHS=y
CONFIG_FB=y
CONFIG_DRM_FBDEV_EMULATION=y
CONFIG_FONT_SUPPORT=y
CONFIG_FONT_8x16=y

# Input (keyboard + mouse for GUI)
CONFIG_INPUT=y
CONFIG_INPUT_EVDEV=y
CONFIG_INPUT_KEYBOARD=y
CONFIG_KEYBOARD_ATKBD=y
CONFIG_SERIO=y
CONFIG_SERIO_I8042=y
CONFIG_INPUT_MOUSE=y
CONFIG_MOUSE_PS2=y
CONFIG_HID=y
CONFIG_HID_GENERIC=y

# USB (for HID devices)
CONFIG_USB=y
CONFIG_USB_SUPPORT=y
CONFIG_USB_HID=y
CONFIG_USB_XHCI_HCD=y
CONFIG_USB_EHCI_HCD=y

# Disable unnecessary stuff
CONFIG_MODULES=n
CONFIG_NETWORK_FILESYSTEMS=n
CONFIG_SOUND=n
CONFIG_WLAN=n
CONFIG_WIRELESS=n
EOF

# Merge configs
make O="$BUILD_DIR" olddefconfig

echo "Compiling kernel (this takes a few minutes)..."
make O="$BUILD_DIR" -j$(nproc) bzImage

echo "Building initramfs..."
/forge/foundry/build_initramfs.sh "$BUILD_DIR"

echo ""
echo "Build complete!"
echo "  Kernel: $IMAGES_DIR/vmlinuz"
echo "  Initrd: $IMAGES_DIR/initramfs.cpio.gz"
echo "  Kernel: $IMAGES_DIR/vmlinuz"
echo "  Initrd: $IMAGES_DIR/initramfs.cpio.gz"
echo ""
echo "To boot manually:"
echo "  qemu-system-x86_64 -kernel $IMAGES_DIR/vmlinuz -initrd $IMAGES_DIR/initramfs.cpio.gz -nographic -append 'console=ttyS0'"
