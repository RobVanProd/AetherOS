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
# Distros vary: qemu-system-i386 (Debian/Ubuntu), or only x86_64 binary present.
if command -v qemu-system-i386 >/dev/null 2>&1; then
  echo "[ok]      qemu-system-i386 -> $(command -v qemu-system-i386)"
elif command -v qemu-system-x86_64 >/dev/null 2>&1; then
  echo "[ok]      qemu-system-x86_64 -> $(command -v qemu-system-x86_64)"
else
  echo "[missing] qemu-system-i386  (hint: sudo apt-get install -y qemu-system-x86)" >&2
  missing=1
fi

echo
if [ "$missing" -eq 1 ]; then
  echo "One or more dependencies are missing." >&2
  exit 2
fi

echo "All required deps found."
