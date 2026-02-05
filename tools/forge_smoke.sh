#!/usr/bin/env bash
set -euo pipefail

# Lightweight smoke check for Forge.
# Usage (from repo root):
#   ./tools/forge_smoke.sh

ROOT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)

cd "${ROOT_DIR}/forge"

if ! command -v cargo >/dev/null 2>&1; then
  echo "error: cargo not found (Rust toolchain missing)." >&2
  echo "hint: install Rust (https://rustup.rs) then re-run: make forge-test" >&2
  exit 1
fi

# --locked enforces Cargo.lock consistency/reproducibility.
cargo test --locked
