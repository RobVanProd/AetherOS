#!/bin/bash
set -euo pipefail
export PATH="$HOME/.cargo/bin:$PATH"

echo "=== Fix 1: Generate Cargo.lock for forge ==="
cd /home/rob/aeternum/AetherOS/forge
cargo generate-lockfile 2>&1
echo "--- Forge lockfile generated ---"

echo ""
echo "=== Fix 1: Verify forge tests ==="
cargo test --locked 2>&1
echo "--- Forge tests complete ---"

echo ""
echo "=== Fix 2: Verify nebula cargo check ==="
cd /home/rob/aeternum/AetherOS/nebula
cargo check 2>&1 || echo "--- Nebula check had errors (see above) ---"

echo ""
echo "=== Fix 3: Verify Makefile ==="
cd /home/rob/aeternum/AetherOS
make forge-test 2>&1
echo "--- Makefile test complete ---"
