#!/usr/bin/env bash
set -euo pipefail

# Runs aetherd + aurorad stubs (Ctrl+C to stop each).

echo "Run in two terminals:"
echo "  (1) cd forge && cargo run -p aetherd"
echo "  (2) cd forge && cargo run -p aurorad"
echo
echo "Then test:"
echo "  curl --unix-socket /tmp/aetherd.sock http://localhost/v0/health"
echo "  curl --unix-socket /tmp/aurorad.sock http://localhost/v0/health"
echo "  curl --unix-socket /tmp/aurorad.sock -H 'Content-Type: application/json' -d '{\"job_type\":\"predict_next_state\"}' http://localhost/v0/jobs"
