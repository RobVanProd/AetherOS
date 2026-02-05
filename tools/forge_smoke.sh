#!/usr/bin/env bash
set -euo pipefail

# Lightweight smoke check for Forge.
# Usage (from repo root):
#   ./tools/forge_smoke.sh

ROOT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)

cd "${ROOT_DIR}/forge"

# --locked enforces Cargo.lock consistency/reproducibility.
cargo test --locked
