#!/usr/bin/env bash
# demo_full_stack.sh — Full-stack AetherOS daemon demo
#
# Starts all three daemons (aetherd, aurorad, cfcd) and exercises
# the complete request path: client → aurorad → cfcd → prediction → audit.
#
# Usage:
#   ./demo_full_stack.sh /path/to/model_final.pt
#   ./demo_full_stack.sh  # uses default checkpoint path

set -euo pipefail

CHECKPOINT="${1:-/home/rob/jepaworlddiffusionlm/internal_world_model/checkpoints_ssv2_h1_baseline_20260204_212814/model_final.pt}"
FORGE_DIR="$(cd "$(dirname "$0")/.." && pwd)"
CFCD_DIR="$(cd "$(dirname "$0")" && pwd)"

# Sockets
AETHERD_SOCK="/tmp/aetherd.sock"
AURORAD_SOCK="/tmp/aurorad.sock"
CFCD_SOCK="/tmp/cfcd.sock"

# Cleanup function
PIDS=()
cleanup() {
    echo ""
    echo "=== Shutting down daemons ==="
    for pid in "${PIDS[@]}"; do
        kill "$pid" 2>/dev/null || true
    done
    wait "${PIDS[@]}" 2>/dev/null || true
    rm -f "$AETHERD_SOCK" "$AURORAD_SOCK" "$CFCD_SOCK"
    echo "Done."
}
trap cleanup EXIT

echo "============================================================"
echo "  AetherOS Full-Stack Demo"
echo "  aetherd (audit) → aurorad (routing) → cfcd (model runtime)"
echo "============================================================"
echo ""

# 1. Start cfcd (Python model daemon)
echo "--- Starting cfcd (CFC-JEPA model runtime) ---"
python3 "$CFCD_DIR/cfcd_server.py" --checkpoint "$CHECKPOINT" --socket "$CFCD_SOCK" &
PIDS+=($!)
sleep 5  # Model loading takes a few seconds

# 2. Start aetherd (Rust audit daemon)
echo "--- Starting aetherd (audit/policy) ---"
"$FORGE_DIR/target/debug/aetherd" &
PIDS+=($!)
sleep 1

# 3. Start aurorad (Rust routing daemon)
echo "--- Starting aurorad (job routing) ---"
CFCD_SOCKET="$CFCD_SOCK" "$FORGE_DIR/target/debug/aurorad" &
PIDS+=($!)
sleep 1

echo ""
echo "============================================================"
echo "  All daemons running. Testing endpoints..."
echo "============================================================"
echo ""

# Test 1: Health checks
echo "=== 1. Health Checks ==="
echo "  cfcd:"
curl -s --unix-socket "$CFCD_SOCK" http://localhost/v0/health | python3 -m json.tool
echo ""
echo "  aurorad:"
curl -s --unix-socket "$AURORAD_SOCK" http://localhost/v0/health | python3 -m json.tool
echo ""
echo "  aetherd:"
curl -s --unix-socket "$AETHERD_SOCK" http://localhost/v0/health | python3 -m json.tool
echo ""

# Test 2: Predict future OS state via aurorad → cfcd
echo "=== 2. Predict Future State (aurorad → cfcd) ==="
curl -s --unix-socket "$AURORAD_SOCK" \
    -X POST -d '{"job_type":"predict_next_state"}' \
    http://localhost/v0/jobs | python3 -c "
import sys, json
data = json.load(sys.stdin)
print(f'  Job ID: {data[\"job_id\"]}')
print(f'  Job Type: {data[\"job_type\"]}')
r = data['result']
print(f'  Latency: {r.get(\"latency_ms\", \"?\")}ms')
pred = r.get('prediction', [])
print(f'  Prediction dims: {len(pred)}')
print(f'  Prediction sample: [{pred[0]:.4f}, {pred[1]:.4f}, ..., {pred[-1]:.4f}]')
gates = r.get('gate_stats', {})
for layer, stats in gates.items():
    print(f'  Gate {layer}: mean={stats[\"mean\"]:.4f} range=[{stats[\"min\"]:.4f}, {stats[\"max\"]:.4f}]')
"
echo ""

# Test 3: Encode OS state via aurorad → cfcd
echo "=== 3. Encode OS State (aurorad → cfcd) ==="
curl -s --unix-socket "$AURORAD_SOCK" \
    -X POST -d '{"job_type":"encode_state"}' \
    http://localhost/v0/jobs | python3 -c "
import sys, json
data = json.load(sys.stdin)
r = data['result']
emb = r.get('embedding', [])
print(f'  Input dim: {r.get(\"input_dim\", \"?\")}')
print(f'  Output dim: {r.get(\"output_dim\", \"?\")}')
print(f'  Embedding sample: [{emb[0]:.4f}, {emb[1]:.4f}, ..., {emb[-1]:.4f}]')
"
echo ""

# Test 4: Model introspection via aurorad → cfcd
echo "=== 4. Model Introspection (aurorad → cfcd) ==="
curl -s --unix-socket "$AURORAD_SOCK" \
    -X POST -d '{"job_type":"introspect"}' \
    http://localhost/v0/jobs | python3 -c "
import sys, json
data = json.load(sys.stdin)
r = data['result']
m = r['model']
print(f'  Model: {m[\"param_count\"]:,} params on {m[\"device\"]}')
print(f'  Config: encoder={m[\"encoder_dim\"]}, hidden={m[\"hidden_dim\"]}, CFC={m[\"use_cfc\"]}')
print(f'  Weight version: {m[\"weight_version\"]}')
l = r['learning']
print(f'  Learning: enabled={l[\"enabled\"]}, observations={l[\"total_observations\"]}')
print(f'  Predictions: {r[\"predictions\"][\"total_predictions\"]} total')
"
echo ""

# Test 5: Audit logging
echo "=== 5. Audit Logging (aetherd) ==="
curl -s --unix-socket "$AETHERD_SOCK" \
    -X POST -d '{"action":"predict","source":"demo","timestamp":"now"}' \
    http://localhost/v0/audit | python3 -m json.tool
echo ""

# Test 6: Policy check
echo "=== 6. Policy Check (aetherd) ==="
curl -s --unix-socket "$AETHERD_SOCK" \
    -X POST -d '{"action":"update_weights","user":"aurorad"}' \
    http://localhost/v0/policy/check | python3 -m json.tool
echo ""

# Test 7: Direct cfcd proxy
echo "=== 7. Direct Model Proxy (aurorad /v0/model/*) ==="
curl -s --unix-socket "$AURORAD_SOCK" \
    http://localhost/v0/model/health | python3 -c "
import sys, json
data = json.load(sys.stdin)
print(f'  Service: {data.get(\"service\", \"?\")} v{data.get(\"version\", \"?\")}')
print(f'  Params: {data.get(\"param_count\", 0):,}')
print(f'  Uptime: {data.get(\"uptime_seconds\", 0)}s')
"
echo ""

echo "============================================================"
echo "  Full-Stack Demo Complete!"
echo ""
echo "  Architecture:"
echo "    client → aurorad (job routing) → cfcd (CFC-JEPA runtime)"
echo "    client → aetherd (audit + policy)"
echo ""
echo "  For live learning demo, run:"
echo "    python3 $CFCD_DIR/demo_e2e.py --checkpoint $CHECKPOINT"
echo "============================================================"
echo ""
echo "Press Ctrl+C to stop all daemons..."
wait
