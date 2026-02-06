#!/bin/bash
#
# Quick boot test for Aether OS
# Run this after building kernel + initramfs
#
set -e

IMAGES_DIR="${1:-./images}"
KERNEL="$IMAGES_DIR/vmlinuz"
INITRD="$IMAGES_DIR/initramfs.cpio.gz"

# Check files exist
if [[ ! -f "$KERNEL" ]]; then
    echo "Error: Kernel not found at $KERNEL"
    exit 1
fi

if [[ ! -f "$INITRD" ]]; then
    echo "Error: Initramfs not found at $INITRD"
    exit 1
fi

echo "Booting Aether OS..."
echo "  Kernel: $KERNEL"
echo "  Initrd: $INITRD"
echo ""
echo "Press Ctrl+A, X to exit QEMU"
echo ""

# Check for KVM
KVM_ARGS=""
if [[ -e /dev/kvm ]]; then
    KVM_ARGS="-enable-kvm -cpu host"
    echo "(KVM acceleration enabled)"
else
    KVM_ARGS="-cpu qemu64"
    echo "(KVM not available, using emulation)"
fi

# Boot with networking enabled
qemu-system-x86_64 \
    $KVM_ARGS \
    -m 512 \
    -kernel "$KERNEL" \
    -initrd "$INITRD" \
    -append "console=ttyS0 quiet loglevel=3" \
    -nographic \
    -netdev user,id=net0,hostfwd=tcp::2222-:22 \
    -device e1000,netdev=net0 \
    -no-reboot
