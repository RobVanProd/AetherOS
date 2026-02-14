#!/usr/bin/env bash
# build_initramfs.sh — Build AetherOS initramfs with Aether daemons + Nebula TUI
#
# Usage:
#   ./tools/build_initramfs.sh              # Build initramfs into build/
#   ./tools/build_initramfs.sh /path/to/dir # Custom output directory

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BUILD_DIR="${1:-$ROOT/build}"
INITRAMFS_DIR="$BUILD_DIR/initramfs"
OUTPUT="$BUILD_DIR/initramfs.cpio.gz"
FORGE_DIR="$ROOT/forge"
FORGE_ORIGINAL="$ROOT/the_forge_original"

echo "=== Building AetherOS initramfs ==="

# Clean and create directory structure
rm -rf "$INITRAMFS_DIR"
mkdir -p "$INITRAMFS_DIR"/{bin,sbin,etc,proc,sys,dev,tmp,run,var,root,usr/bin,usr/sbin,mnt,home}

# ---- BusyBox ----
BUSYBOX_BIN="$INITRAMFS_DIR/bin/busybox"
if [[ -f "$FORGE_ORIGINAL/cache/busybox" ]]; then
    echo "  [busybox] Copying from forge cache"
    cp "$FORGE_ORIGINAL/cache/busybox" "$BUSYBOX_BIN"
elif [[ -f "$BUILD_DIR/cache/busybox" ]]; then
    echo "  [busybox] Copying from build cache"
    cp "$BUILD_DIR/cache/busybox" "$BUSYBOX_BIN"
else
    echo "  [busybox] Downloading v1.35.0 (static musl)"
    mkdir -p "$BUILD_DIR/cache"
    wget -q "https://busybox.net/downloads/binaries/1.35.0-x86_64-linux-musl/busybox" -O "$BUSYBOX_BIN"
    cp "$BUSYBOX_BIN" "$BUILD_DIR/cache/busybox"
fi
chmod +x "$BUSYBOX_BIN"

# BusyBox symlinks
cd "$INITRAMFS_DIR/bin"
for cmd in sh ash cat cp mv rm mkdir rmdir ls ln chmod chown \
           grep sed awk cut head tail sort uniq wc tr \
           echo printf test true false sleep date hostname uname \
           mount umount ps top kill killall free df du \
           ping ip ifconfig wget vi tar gzip gunzip \
           clear dmesg mdev poweroff reboot halt; do
    ln -sf busybox "$cmd" 2>/dev/null || true
done

cd "$INITRAMFS_DIR/sbin"
for cmd in init mdev ifconfig route ip halt poweroff reboot syslogd udhcpc; do
    ln -sf ../bin/busybox "$cmd" 2>/dev/null || true
done

# ---- Aether static binaries ----
MUSL_TARGET="x86_64-unknown-linux-musl"
MUSL_RELEASE="$FORGE_DIR/target/$MUSL_TARGET/release"

# Copy aetherd if built
if [[ -f "$MUSL_RELEASE/aetherd" ]]; then
    echo "  [aetherd] Installing static binary"
    cp "$MUSL_RELEASE/aetherd" "$INITRAMFS_DIR/sbin/aetherd"
    chmod +x "$INITRAMFS_DIR/sbin/aetherd"
else
    echo "  [aetherd] Not found (build with: cargo build --target $MUSL_TARGET --release)"
fi

# Copy aurorad if built
if [[ -f "$MUSL_RELEASE/aurorad" ]]; then
    echo "  [aurorad] Installing static binary"
    cp "$MUSL_RELEASE/aurorad" "$INITRAMFS_DIR/sbin/aurorad"
    chmod +x "$INITRAMFS_DIR/sbin/aurorad"
else
    echo "  [aurorad] Not found (build with: cargo build --target $MUSL_TARGET --release)"
fi

# Copy nebula-tui if built
if [[ -f "$MUSL_RELEASE/nebula-tui" ]]; then
    echo "  [nebula] Installing static binary"
    cp "$MUSL_RELEASE/nebula-tui" "$INITRAMFS_DIR/bin/nebula"
    chmod +x "$INITRAMFS_DIR/bin/nebula"
else
    echo "  [nebula] Not found (build with: cargo build --target $MUSL_TARGET --release)"
fi

# Copy nebula-fb (framebuffer GUI) if built
if [[ -f "$MUSL_RELEASE/nebula-fb" ]]; then
    echo "  [nebula-fb] Installing static binary"
    cp "$MUSL_RELEASE/nebula-fb" "$INITRAMFS_DIR/bin/nebula-fb"
    chmod +x "$INITRAMFS_DIR/bin/nebula-fb"
else
    echo "  [nebula-fb] Not found (optional — graphical mode requires it)"
fi

# ---- Sound files ----
SOUNDS_SRC="$ROOT/assets/sounds"
if [[ -f "$SOUNDS_SRC/post.wav" ]]; then
    echo "  [sounds] Installing sound files"
    mkdir -p "$INITRAMFS_DIR/usr/share/sounds"
    cp "$SOUNDS_SRC/post.wav" "$INITRAMFS_DIR/usr/share/sounds/post.wav"
else
    echo "  [sounds] No sound files found (optional)"
fi

# ---- Init script ----
echo "  [init] Installing Aether init"
cp "$ROOT/aether_init/init" "$INITRAMFS_DIR/init"
chmod +x "$INITRAMFS_DIR/init"

# ---- Config files ----
echo "root:x:0:0:root:/root:/bin/sh" > "$INITRAMFS_DIR/etc/passwd"
echo "root:x:0:" > "$INITRAMFS_DIR/etc/group"
echo "aether" > "$INITRAMFS_DIR/etc/hostname"

# DHCP script
cat > "$INITRAMFS_DIR/etc/udhcpc.script" << 'DHCP'
#!/bin/sh
case "$1" in
    deconfig) ip addr flush dev $interface; ip link set $interface up;;
    bound|renew) ip addr add $ip/$mask dev $interface
        [ -n "$router" ] && ip route add default via $router dev $interface
        [ -n "$dns" ] && echo "nameserver $dns" > /etc/resolv.conf;;
esac
DHCP
chmod +x "$INITRAMFS_DIR/etc/udhcpc.script"
touch "$INITRAMFS_DIR/etc/resolv.conf"

# ---- Device nodes (may need root, skip if not available) ----
cd "$INITRAMFS_DIR/dev"
if mknod -m 622 console c 5 1 2>/dev/null; then
    mknod -m 666 null c 1 3 2>/dev/null || true
    mknod -m 666 zero c 1 5 2>/dev/null || true
    mknod -m 666 tty c 5 0 2>/dev/null || true
else
    echo "  [dev] Skipping device nodes (needs root). Kernel devtmpfs will handle it."
fi

# ---- Pack ----
echo "  [pack] Creating initramfs.cpio.gz"
cd "$INITRAMFS_DIR"
find . -print0 | cpio --null -ov --format=newc 2>/dev/null | gzip -9 > "$OUTPUT"

echo ""
echo "=== Initramfs built ==="
echo "  Output: $OUTPUT ($(du -h "$OUTPUT" | cut -f1))"
echo "  BusyBox: $(ls -lh bin/busybox | awk '{print $5}')"
[[ -f sbin/aetherd ]] && echo "  aetherd:   $(ls -lh sbin/aetherd | awk '{print $5}')" || true
[[ -f sbin/aurorad ]] && echo "  aurorad:   $(ls -lh sbin/aurorad | awk '{print $5}')" || true
[[ -f bin/nebula ]] && echo "  nebula:    $(ls -lh bin/nebula | awk '{print $5}')" || true
[[ -f bin/nebula-fb ]] && echo "  nebula-fb: $(ls -lh bin/nebula-fb | awk '{print $5}')" || true
