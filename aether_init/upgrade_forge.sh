#!/bin/bash
#
# Upgrade The Forge with Aether init v0.2
# Run this from your the_forge directory
#
set -e

FORGE_DIR="${1:-.}"

if [[ ! -d "$FORGE_DIR/foundry" ]]; then
    echo "Error: Run this from the_forge directory, or pass path as argument"
    exit 1
fi

echo "Upgrading The Forge with Aether init v0.2..."

# Backup original
if [[ -f "$FORGE_DIR/foundry/build_kernel.sh" ]]; then
    cp "$FORGE_DIR/foundry/build_kernel.sh" "$FORGE_DIR/foundry/build_kernel.sh.bak"
fi

# Create new initramfs builder
cat > "$FORGE_DIR/foundry/build_initramfs.sh" << 'SCRIPT'
#!/bin/bash
set -e
BUILD_DIR="${1:-/forge/build}"
INITRAMFS_DIR="$BUILD_DIR/initramfs"
OUTPUT="$BUILD_DIR/initramfs.cpio.gz"

echo "Building Aether initramfs v0.2..."

rm -rf "$INITRAMFS_DIR"
mkdir -p "$INITRAMFS_DIR"/{bin,sbin,etc,proc,sys,dev,tmp,run,var,root,usr/bin,usr/sbin,mnt,home}

# BusyBox
BUSYBOX_BIN="$INITRAMFS_DIR/bin/busybox"
if [[ -f "/forge/cache/busybox" ]]; then
    cp /forge/cache/busybox "$BUSYBOX_BIN"
else
    wget -q "https://busybox.net/downloads/binaries/1.35.0-x86_64-linux-musl/busybox" -O "$BUSYBOX_BIN"
    mkdir -p /forge/cache
    cp "$BUSYBOX_BIN" /forge/cache/busybox
fi
chmod +x "$BUSYBOX_BIN"

# Symlinks
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

# Init script
cat > "$INITRAMFS_DIR/init" << 'INIT'
#!/bin/sh
mount -t proc none /proc
mount -t sysfs none /sys
mount -t devtmpfs none /dev 2>/dev/null || mknod -m 622 /dev/console c 5 1
mkdir -p /dev/pts /dev/shm
mount -t devpts devpts /dev/pts 2>/dev/null
mount -t tmpfs tmpfs /dev/shm 2>/dev/null
mount -t tmpfs tmpfs /tmp
mount -t tmpfs tmpfs /run
mount -t tmpfs tmpfs /var
mkdir -p /var/log /var/tmp

hostname aether
echo "3 3 3 3" > /proc/sys/kernel/printk 2>/dev/null
echo /sbin/mdev > /proc/sys/kernel/hotplug 2>/dev/null
mdev -s 2>/dev/null

ip link set lo up 2>/dev/null
ip addr add 127.0.0.1/8 dev lo 2>/dev/null

[ -d /sys/class/net/eth0 ] && {
    ip link set eth0 up 2>/dev/null
    udhcpc -i eth0 -s /etc/udhcpc.script -q -n -b 2>/dev/null &
}

export HOME=/root PATH=/bin:/sbin:/usr/bin:/usr/sbin TERM=linux
export PS1='\033[1;36maether\033[0m:\033[1;34m\w\033[0m\$ '

cat > /bin/sysinfo << 'S'
#!/bin/sh
echo ""; echo "===== AETHER SYSTEM ====="
echo "Kernel:  $(uname -r)"
echo "Uptime:  $(cut -d. -f1 /proc/uptime)s"
echo "CPU:     $(grep -c processor /proc/cpuinfo) cores"
echo "Memory:  $(awk '/MemTotal/{t=$2}/MemFree/{f=$2}END{printf "%.0f/%.0fMB",f/1024,t/1024}' /proc/meminfo)"
ip -br addr 2>/dev/null|grep -v "^lo"||true; echo "========================="
S
chmod +x /bin/sysinfo

cat > /bin/help << 'H'
#!/bin/sh
echo "sysinfo|ps|top|free|df|dmesg|ip addr|ping|vi|poweroff"
H
chmod +x /bin/help

echo "BOOT_SUCCESS"
clear 2>/dev/null||true
echo ""; echo "  AETHER OS v0.2 | $(uname -r)"; echo "  Type 'help' for commands"; echo ""
cd /root; exec /bin/sh
INIT
chmod +x "$INITRAMFS_DIR/init"

# Config files
echo "root:x:0:0:root:/root:/bin/sh" > "$INITRAMFS_DIR/etc/passwd"
echo "root:x:0:" > "$INITRAMFS_DIR/etc/group"
echo "aether" > "$INITRAMFS_DIR/etc/hostname"
cat > "$INITRAMFS_DIR/etc/udhcpc.script" << 'D'
#!/bin/sh
case "$1" in
    deconfig) ip addr flush dev $interface; ip link set $interface up;;
    bound|renew) ip addr add $ip/$mask dev $interface
        [ -n "$router" ] && ip route add default via $router dev $interface
        [ -n "$dns" ] && echo "nameserver $dns" > /etc/resolv.conf;;
esac
D
chmod +x "$INITRAMFS_DIR/etc/udhcpc.script"
touch "$INITRAMFS_DIR/etc/resolv.conf"

# Device nodes
cd "$INITRAMFS_DIR/dev"
mknod -m 622 console c 5 1 2>/dev/null || true
mknod -m 666 null c 1 3 2>/dev/null || true
mknod -m 666 zero c 1 5 2>/dev/null || true
mknod -m 666 tty c 5 0 2>/dev/null || true

# Pack
cd "$INITRAMFS_DIR"
find . -print0 | cpio --null -ov --format=newc 2>/dev/null | gzip -9 > "$OUTPUT"
echo "Initramfs: $OUTPUT ($(du -h "$OUTPUT" | cut -f1))"
SCRIPT

chmod +x "$FORGE_DIR/foundry/build_initramfs.sh"

# Update Foundry to use new initramfs builder
if grep -q "Creating minimal initramfs" "$FORGE_DIR/foundry/build_kernel.sh"; then
    # Replace the inline initramfs build with a call to our script
    sed -i '/echo "Creating minimal initramfs..."/,/echo "Build complete!"/c\
echo "Building initramfs..."\
/forge/foundry/build_initramfs.sh "$BUILD_DIR"\
\
echo ""\
echo "Build complete!"\
echo "  Kernel: $IMAGES_DIR/vmlinuz"\
echo "  Initrd: $IMAGES_DIR/initramfs.cpio.gz"' "$FORGE_DIR/foundry/build_kernel.sh"
fi

echo ""
echo "Upgrade complete!"
echo ""
echo "Changes:"
echo "  - Added foundry/build_initramfs.sh (new interactive init)"
echo "  - Backed up original build_kernel.sh"
echo ""
echo "New boot will drop you into an interactive shell with:"
echo "  - sysinfo, help commands"
echo "  - Networking support (DHCP)"
echo "  - 40+ standard Unix commands"
echo ""
echo "Rebuild and test:"
echo "  docker-compose run foundry"
echo "  docker-compose run crucible"
