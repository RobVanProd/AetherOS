#!/usr/bin/env bash
# run_qemu.sh — Boot AetherOS in QEMU
#
# Usage:
#   ./tools/run_qemu.sh                  # Boot with defaults
#   ./tools/run_qemu.sh --with-cfcd      # Also start cfcd on host
#   Press Ctrl+A, X to exit QEMU

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
KERNEL="$ROOT/the_forge_original/images/vmlinuz"
INITRD="$ROOT/build/initramfs.cpio.gz"
CFCD_PID=""

# Check for kernel
if [[ ! -f "$KERNEL" ]]; then
    echo "Error: Kernel not found at $KERNEL"
    echo "Run the Forge pipeline first, or provide a vmlinuz."
    exit 1
fi

# Check for initramfs
if [[ ! -f "$INITRD" ]]; then
    echo "Error: Initramfs not found at $INITRD"
    echo "Run: make build-initramfs"
    exit 1
fi

# Cleanup on exit
cleanup() {
    if [[ -n "$CFCD_PID" ]]; then
        echo "[host] Stopping cfcd (PID $CFCD_PID)"
        kill "$CFCD_PID" 2>/dev/null || true
        wait "$CFCD_PID" 2>/dev/null || true
    fi
}
trap cleanup EXIT

# Optionally start cfcd on host
if [[ "${1:-}" == "--with-cfcd" ]]; then
    CHECKPOINT="/home/rob/jepaworlddiffusionlm/internal_world_model/checkpoints_ssv2_h1_baseline_20260204_212814/model_final.pt"
    if [[ -f "$CHECKPOINT" ]]; then
        echo "[host] Starting cfcd on TCP port 9100..."
        python3 "$ROOT/forge/cfcd/cfcd_server.py" \
            --checkpoint "$CHECKPOINT" \
            --socket /tmp/cfcd.sock \
            --tcp-port 9100 &
        CFCD_PID=$!
        sleep 4
        echo "[host] cfcd running (PID $CFCD_PID)"
    else
        echo "[host] Warning: Checkpoint not found, skipping cfcd"
    fi
fi

echo ""
echo "=========================================="
echo "  Booting AetherOS"
echo "  Kernel: $(basename $KERNEL)"
echo "  Initrd: $(basename $INITRD) ($(du -h "$INITRD" | cut -f1))"
echo "=========================================="
echo "  Press Ctrl+A, X to exit QEMU"
echo ""

# KVM acceleration
KVM_ARGS=""
if [[ -w /dev/kvm ]]; then
    KVM_ARGS="-enable-kvm -cpu host"
else
    KVM_ARGS="-cpu qemu64"
fi

# Boot
qemu-system-x86_64 \
    $KVM_ARGS \
    -m 512 \
    -kernel "$KERNEL" \
    -initrd "$INITRD" \
    -append "console=ttyS0 quiet loglevel=3" \
    -nographic \
    -no-reboot \
    -netdev user,id=net0,hostfwd=tcp::2222-:22 \
    -device e1000,netdev=net0
