#!/bin/bash
#
# The Foundry - Bespoke Kernel Builder
# Builds a custom kernel tailored to a specific machine's hardware.
#
set -e

KERNEL_SRC="/forge/kernel_src"
BUILD_DIR="/forge/build"
IMAGES_DIR="/forge/images"
MACHINES_DIR="/forge/machines"
FOUNDRY_DIR="/forge/foundry"

# Parse arguments
MACHINE_ID="${1:-}"

if [[ -z "$MACHINE_ID" ]]; then
    echo "Usage: $0 <machine_id>"
    echo ""
    echo "Builds a bespoke kernel for the specified machine configuration."
    exit 1
fi

MACHINE_JSON="$MACHINES_DIR/${MACHINE_ID}.json"

if [[ ! -f "$MACHINE_JSON" ]]; then
    echo "ERROR: Machine config not found: $MACHINE_JSON"
    exit 1
fi

echo "========================================"
echo "  The Foundry: Bespoke Kernel Build"
echo "========================================"
echo "Machine ID: $MACHINE_ID"
echo "Config: $MACHINE_JSON"
echo ""

# Read machine metadata
PROFILE=$(cat "$MACHINE_JSON" | python3 -c "import sys, json; print(json.load(sys.stdin).get('profile', 'unknown'))")
CPU_CORES=$(cat "$MACHINE_JSON" | python3 -c "import sys, json; print(json.load(sys.stdin)['cpu']['cores'])")
MEMORY_MB=$(cat "$MACHINE_JSON" | python3 -c "import sys, json; print(json.load(sys.stdin)['memory_mb'])")

echo "Profile: $PROFILE"
echo "CPU: ${CPU_CORES} cores"
echo "Memory: ${MEMORY_MB}MB"
echo ""

# Create build directory
BUILD_MACHINE_DIR="$BUILD_DIR/${MACHINE_ID}"
mkdir -p "$BUILD_MACHINE_DIR" "$IMAGES_DIR"

cd "$KERNEL_SRC"

# Generate bespoke config
echo "Generating bespoke kernel config..."
python3 "$FOUNDRY_DIR/generate_config.py" "$MACHINE_JSON" > "$BUILD_MACHINE_DIR/.config.fragment"

# Start with allnoconfig, then apply our fragment
make O="$BUILD_MACHINE_DIR" allnoconfig

# Append our config fragment
cat "$BUILD_MACHINE_DIR/.config.fragment" >> "$BUILD_MACHINE_DIR/.config"

# Resolve dependencies
make O="$BUILD_MACHINE_DIR" olddefconfig

echo "Config generated: $BUILD_MACHINE_DIR/.config"
echo ""

# Build kernel
echo "Compiling bespoke kernel (using $(nproc) cores)..."
time make O="$BUILD_MACHINE_DIR" -j$(nproc) 2>&1 | tail -20

# Copy kernel image
KERNEL_IMAGE="$BUILD_MACHINE_DIR/arch/x86/boot/bzImage"
if [[ -f "$KERNEL_IMAGE" ]]; then
    cp "$KERNEL_IMAGE" "$IMAGES_DIR/vmlinuz-${MACHINE_ID}"
    echo ""
    echo "✓ Kernel built successfully"
    echo "  Output: $IMAGES_DIR/vmlinuz-${MACHINE_ID}"
    echo "  Size: $(du -h "$IMAGES_DIR/vmlinuz-${MACHINE_ID}" | cut -f1)"
else
    echo "ERROR: Kernel build failed - bzImage not found"
    exit 1
fi

# Save build metadata
cat > "$IMAGES_DIR/build_info-${MACHINE_ID}.json" << EOF
{
    "machine_id": "$MACHINE_ID",
    "profile": "$PROFILE",
    "kernel_version": "$(cat $KERNEL_SRC/Makefile | grep '^VERSION = ' | cut -d' ' -f3).$(cat $KERNEL_SRC/Makefile | grep '^PATCHLEVEL = ' | cut -d' ' -f3)",
    "build_timestamp": "$(date -Iseconds)",
    "config_file": "$BUILD_MACHINE_DIR/.config",
    "kernel_image": "vmlinuz-${MACHINE_ID}"
}
EOF

echo ""
echo "Build complete!"
