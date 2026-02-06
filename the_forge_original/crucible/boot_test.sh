#!/bin/bash
#
# The Crucible
# Automated boot testing via QEMU.
#
set -e

IMAGES_DIR="/forge/images"
MACHINES_DIR="/forge/machines"
RESULTS_DIR="/forge/results"
TIMEOUT=15  # seconds to wait for boot

echo "The Crucible: Automated boot testing"

mkdir -p "$RESULTS_DIR"

# Check if we have a kernel to boot
if [[ ! -f "$IMAGES_DIR/vmlinuz" ]]; then
    echo "ERROR: No kernel found. Run foundry first."
    exit 1
fi

# Function to test a single boot
test_boot() {
    local machine_id=$1
    local machine_config=$2
    local result_file="$RESULTS_DIR/${machine_id}.json"
    local log_file="$RESULTS_DIR/${machine_id}.log"
    
    echo "Testing machine: $machine_id"
    
    # Extract QEMU args from config (if provided)
    # For prototype, use simple defaults
    
    local start_time=$(date +%s)
    
    # Run QEMU with timeout, capture output
    timeout "$TIMEOUT" qemu-system-x86_64 \
        -kernel "$IMAGES_DIR/vmlinuz" \
        -initrd "$IMAGES_DIR/initramfs.cpio.gz" \
        -append "console=ttyS0 quiet" \
        -m 256 \
        -nographic \
        -no-reboot \
        -device isa-debug-exit,iobase=0xf4,iosize=0x04 \
        2>&1 | tee "$log_file" || true
    
    local end_time=$(date +%s)
    local duration=$((end_time - start_time))
    
    # Analyze results
    local status="unknown"
    local boot_success=false
    
    if grep -q "BOOT_SUCCESS" "$log_file"; then
        status="success"
        boot_success=true
    elif grep -q "Kernel panic" "$log_file"; then
        status="kernel_panic"
    elif grep -q "timeout" "$log_file" || [[ $duration -ge $TIMEOUT ]]; then
        status="timeout"
    else
        status="unknown_failure"
    fi
    
    # Write result
    cat > "$result_file" << EOF
{
    "machine_id": "$machine_id",
    "status": "$status",
    "boot_success": $boot_success,
    "duration_seconds": $duration,
    "log_file": "${machine_id}.log",
    "timestamp": "$(date -Iseconds)"
}
EOF
    
    if $boot_success; then
        echo "  ✓ PASS ($duration seconds)"
    else
        echo "  ✗ FAIL: $status"
    fi
    
    return $( $boot_success && echo 0 || echo 1 )
}

# Run tests
echo ""
echo "Running boot tests..."
echo "----------------------------------------"

total=0
passed=0

# If we have machine configs, test each one
if [[ -f "$MACHINES_DIR/index.json" ]]; then
    machines=$(cat "$MACHINES_DIR/index.json" | python3 -c "import sys,json; print(' '.join(json.load(sys.stdin)['machines']))")
    
    for machine_id in $machines; do
        total=$((total + 1))
        if test_boot "$machine_id" "$MACHINES_DIR/${machine_id}.json"; then
            passed=$((passed + 1))
        fi
    done
else
    # No machines defined, just test with defaults
    total=1
    if test_boot "default" ""; then
        passed=1
    fi
fi

echo "----------------------------------------"
echo ""
echo "Results: $passed / $total passed"

# Write summary
cat > "$RESULTS_DIR/summary.json" << EOF
{
    "timestamp": "$(date -Iseconds)",
    "total_tests": $total,
    "passed": $passed,
    "failed": $((total - passed)),
    "pass_rate": $(echo "scale=2; $passed * 100 / $total" | bc)
}
EOF

echo "Results saved to $RESULTS_DIR/"

# Exit with failure if any tests failed
[[ $passed -eq $total ]] || exit 1
