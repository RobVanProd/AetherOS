#!/bin/bash
# Test Aether OS networking in QEMU
set -e

echo "Testing Aether OS Networking..."
echo "================================"
echo ""

# Boot QEMU with network, send commands via serial
timeout 15 qemu-system-x86_64 \
    -kernel images/vmlinuz \
    -initrd images/initramfs.cpio.gz \
    -nographic \
    -append "console=ttyS0" \
    -m 512 \
    -netdev user,id=net0 \
    -device e1000,netdev=net0 \
    2>&1 | tee /tmp/aether_boot.log || true

echo ""
echo "=== Boot Log Analysis ==="
grep -E "(ip addr|inet |DHCP|eth0)" /tmp/aether_boot.log || echo "No network info found"
