#!/usr/bin/env bash
set -euo pipefail

# Simple dependency check for the current QEMU harness.
# NOTE: Today we build/run legacy/MyOS while migration proceeds.

missing=0
need(){
  local bin="$1" pkgHint="$2"
  if ! command -v "$bin" >/dev/null 2>&1; then
    echo "[missing] $bin  (hint: $pkgHint)" >&2
    missing=1
  else
    echo "[ok]      $bin -> $(command -v "$bin")"
  fi
}

echo "AetherOS build/run deps (current harness)"
need make "sudo apt-get install -y make"
need nasm "sudo apt-get install -y nasm"
need qemu-system-i386 "sudo apt-get install -y qemu-system-x86"

echo
if [ "$missing" -eq 1 ]; then
  echo "One or more dependencies are missing." >&2
  exit 2
fi

echo "All required deps found."
