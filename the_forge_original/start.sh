#!/bin/bash
#
# Quick Start: The Forge
# Builds and runs the complete pipeline
#
set -e

# Read version if available
VERSION="1.0.0"
if [[ -f VERSION ]]; then
    VERSION=$(cat VERSION | tr -d '\n')
fi

echo "========================================"
echo "  AETHER OS v${VERSION}"
echo "  The Forge - Automated Build Pipeline"
echo "  Aeternum Labs"
echo "========================================"
echo ""

# Check Docker
if ! command -v docker &> /dev/null; then
    echo "ERROR: Docker is required but not installed."
    exit 1
fi

# Check for KVM support (optional but recommended)
if [[ -e /dev/kvm ]]; then
    echo "✓ KVM acceleration available"
else
    echo "⚠ KVM not available - boot tests will be slower"
fi

echo ""
echo "Step 1: Building Docker image (first run takes ~5 min)..."
docker build -t aeternum/forge:latest .

echo ""
echo "Step 2: Running full pipeline..."
docker run --privileged \
    -v "$(pwd)/results:/forge/results" \
    -v "$(pwd)/images:/forge/images" \
    -v "$(pwd)/machines:/forge/machines" \
    -v "$(pwd)/data:/forge/data" \
    aeternum/forge:latest all

echo ""
echo "========================================"
echo "  BUILD COMPLETE - v${VERSION}"
echo "========================================"
echo ""
echo "Build Artifacts:"
echo "  - images/vmlinuz              Bootable kernel"
echo "  - images/initramfs.cpio.gz    Initial ramdisk"
echo "  - images/build_info.json      Build metadata"
echo ""
echo "Generated Data:"
echo "  - data/driver_manifest.json   Driver database"
echo "  - machines/*.json             Machine configs ($(ls machines/*.json 2>/dev/null | wc -l) total)"
echo "  - results/summary.json        Test results"
echo ""
echo "Test Results:"
if [[ -f results/summary.json ]]; then
    PASSED=$(grep -o '"passed": [0-9]*' results/summary.json | cut -d' ' -f2)
    TOTAL=$(grep -o '"total_tests": [0-9]*' results/summary.json | cut -d' ' -f2)
    RATE=$(grep -o '"pass_rate": [0-9.]*' results/summary.json | cut -d' ' -f2)
    echo "  ✓ Tests passed: ${PASSED}/${TOTAL} (${RATE}%)"
else
    echo "  - No test results available"
fi
echo ""
echo "Quick Start:"
echo "  Boot kernel:  qemu-system-x86_64 -kernel images/vmlinuz -initrd images/initramfs.cpio.gz -nographic"
echo "  Debug shell:  docker run -it --privileged -v \$(pwd):/forge aeternum/forge:latest shell"
echo ""
