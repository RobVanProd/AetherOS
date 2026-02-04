#!/usr/bin/env bash
set -euo pipefail

# AetherOS QEMU harness (v0)
#
# Today this builds/runs legacy/MyOS while we migrate modules into canonical paths.

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

echo "[AetherOS] Running QEMU harness (legacy/MyOS)"

# Preflight dependency check (best-effort)
if [ -x "$ROOT/tools/check_deps.sh" ]; then
  "$ROOT/tools/check_deps.sh" || true
fi

cd "$ROOT/legacy/MyOS"

echo "[AetherOS] Building..."
make

echo "[AetherOS] Booting in QEMU..."
make run
