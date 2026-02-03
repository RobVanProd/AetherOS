#!/usr/bin/env bash
set -euo pipefail

# Placeholder harness: will be wired to canonical kernel paths after migration.
# For now, this points at legacy/MyOS.

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT/legacy/MyOS"

if ! command -v qemu-system-i386 >/dev/null 2>&1; then
  echo "qemu-system-i386 not found. Install qemu-system-x86." >&2
  exit 1
fi

make
make run
