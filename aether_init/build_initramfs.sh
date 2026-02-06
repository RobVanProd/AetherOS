#!/bin/bash
#
# Build Aether OS Initramfs
# Creates a usable interactive environment
#
set -e

BUILD_DIR="${1:-/forge/build}"
INITRAMFS_DIR="$BUILD_DIR/initramfs"
OUTPUT="$BUILD_DIR/initramfs.cpio.gz"

echo "Building Aether initramfs..."

# Clean and create structure
rm -rf "$INITRAMFS_DIR"
mkdir -p "$INITRAMFS_DIR"/{bin,sbin,etc,proc,sys,dev,tmp,run,var,root,usr/bin,usr/sbin,mnt,home}

# ============================================
# BUSYBOX - provides most utilities
# ============================================
echo "[1/5] Installing BusyBox..."

BUSYBOX_URL="https://busybox.net/downloads/binaries/1.35.0-x86_64-linux-musl/busybox"
BUSYBOX_BIN="$INITRAMFS_DIR/bin/busybox"

if [[ -f "/forge/cache/busybox" ]]; then
    cp /forge/cache/busybox "$BUSYBOX_BIN"
else
    wget -q "$BUSYBOX_URL" -O "$BUSYBOX_BIN"
    mkdir -p /forge/cache
    cp "$BUSYBOX_BIN" /forge/cache/busybox
fi
chmod +x "$BUSYBOX_BIN"

# Create all busybox symlinks
cd "$INITRAMFS_DIR/bin"
for cmd in sh ash bash cat cp mv rm mkdir rmdir ls ln chmod chown \
           grep sed awk cut head tail sort uniq wc tr xargs \
           echo printf test true false yes \
           sleep date hostname uname whoami id \
           mount umount mknod \
           ps top kill killall \
           free df du \
           ping ip ifconfig route \
           wget nc telnet \
           vi ed \
           tar gzip gunzip zcat \
           find xargs \
           clear reset \
           dmesg \
           mdev \
           poweroff reboot halt shutdown \
           ; do
    ln -sf busybox "$cmd" 2>/dev/null || true
done

cd "$INITRAMFS_DIR/sbin"
for cmd in init mdev ifconfig route ip halt poweroff reboot \
           switch_root pivot_root \
           syslogd klogd \
           udhcpc \
           ; do
    ln -sf ../bin/busybox "$cmd" 2>/dev/null || true
done

# ============================================
# INIT SYSTEM
# ============================================
echo "[2/5] Installing init system..."

cat > "$INITRAMFS_DIR/init" << 'INIT_EOF'
#!/bin/sh
#
# Aether OS Init v0.2
#

# Mount essential filesystems
mount -t proc none /proc
mount -t sysfs none /sys
mount -t devtmpfs none /dev 2>/dev/null || mknod -m 622 /dev/console c 5 1
mkdir -p /dev/pts /dev/shm
mount -t devpts devpts /dev/pts 2>/dev/null
mount -t tmpfs tmpfs /dev/shm 2>/dev/null

# Writable areas
mount -t tmpfs tmpfs /tmp
mount -t tmpfs tmpfs /run
mount -t tmpfs tmpfs /var
mkdir -p /var/log /var/tmp

# Hostname
hostname aether

# Reduce kernel noise
echo "3 3 3 3" > /proc/sys/kernel/printk 2>/dev/null

# Device hotplug
echo /sbin/mdev > /proc/sys/kernel/hotplug 2>/dev/null
mdev -s 2>/dev/null

# Loopback network
ip link set lo up 2>/dev/null
ip addr add 127.0.0.1/8 dev lo 2>/dev/null

# Try to bring up eth0 if present
if [ -d /sys/class/net/eth0 ]; then
    ip link set eth0 up 2>/dev/null
    # Background DHCP
    udhcpc -i eth0 -s /etc/udhcpc.script -q -n -b 2>/dev/null &
fi

# Environment
export HOME=/root
export PATH=/bin:/sbin:/usr/bin:/usr/sbin
export TERM=linux
export PS1='\033[1;36maether\033[0m:\033[1;34m\w\033[0m\$ '

# Create helper scripts
cat > /bin/sysinfo << 'SYSINFO'
#!/bin/sh
echo ""
echo "===== AETHER SYSTEM INFO ====="
echo "Kernel:  $(uname -r)"
echo "Arch:    $(uname -m)"
echo "Host:    $(hostname)"
echo "Uptime:  $(cut -d. -f1 /proc/uptime)s"
echo ""
echo "CPU:     $(grep -c processor /proc/cpuinfo) cores"
echo "Memory:  $(awk '/MemTotal/{t=$2} /MemFree/{f=$2} END{printf "%.0f MB free / %.0f MB total", f/1024, t/1024}' /proc/meminfo)"
echo ""
echo "Network:"
ip -brief addr 2>/dev/null | grep -v "^lo" || echo "  (none)"
echo "=============================="
SYSINFO
chmod +x /bin/sysinfo

cat > /bin/help << 'HELP'
#!/bin/sh
echo ""
echo "===== AETHER COMMANDS ====="
echo "sysinfo  - system information"
echo "ps       - processes"
echo "top      - process monitor"
echo "free     - memory usage"
echo "df       - disk usage"
echo "dmesg    - kernel log"
echo "ip addr  - network config"
echo "ping     - test network"
echo "vi       - text editor"
echo "poweroff - shutdown"
echo "==========================="
HELP
chmod +x /bin/help

# Signal success for automated testing
echo "BOOT_SUCCESS"

# Banner
clear 2>/dev/null || true
cat << 'BANNER'

   _____          __  .__                  ________    _________
  /  _  \   _____/  |_|  |__   ___________\_____  \  /   _____/
 /  /_\  \_/ __ \   __\  |  \_/ __ \_  __ \/   |   \ \_____  \ 
/    |    \  ___/|  | |   Y  \  ___/|  | \/    |    \/        \
\____|__  /\___  >__| |___|  /\___  >__|  \_______  /_______  /
        \/     \/          \/     \/              \/        \/ 

BANNER
echo "  v0.2.0-prototype | $(uname -r) | $(awk '/MemTotal/{printf "%.0fMB RAM", $2/1024}' /proc/meminfo)"
echo ""
echo "  Type 'help' for commands, 'sysinfo' for system info"
echo ""

# Drop to shell
cd /root
exec /bin/sh
INIT_EOF
chmod +x "$INITRAMFS_DIR/init"

# ============================================
# CONFIGURATION FILES
# ============================================
echo "[3/5] Creating configuration..."

# Basic /etc files
echo "root:x:0:0:root:/root:/bin/sh" > "$INITRAMFS_DIR/etc/passwd"
echo "root:x:0:" > "$INITRAMFS_DIR/etc/group"
echo "aether" > "$INITRAMFS_DIR/etc/hostname"
echo "127.0.0.1 localhost aether" > "$INITRAMFS_DIR/etc/hosts"

# DHCP script for udhcpc
mkdir -p "$INITRAMFS_DIR/etc"
cat > "$INITRAMFS_DIR/etc/udhcpc.script" << 'DHCP'
#!/bin/sh
case "$1" in
    deconfig)
        ip addr flush dev $interface
        ip link set $interface up
        ;;
    bound|renew)
        ip addr add $ip/$mask dev $interface
        if [ -n "$router" ]; then
            ip route add default via $router dev $interface
        fi
        if [ -n "$dns" ]; then
            echo "nameserver $dns" > /etc/resolv.conf
        fi
        ;;
esac
DHCP
chmod +x "$INITRAMFS_DIR/etc/udhcpc.script"

# Empty resolv.conf (DHCP will populate)
touch "$INITRAMFS_DIR/etc/resolv.conf"

# ============================================
# DEVICE NODES (fallback if devtmpfs fails)
# ============================================
echo "[4/5] Creating device nodes..."

cd "$INITRAMFS_DIR/dev"
mknod -m 622 console c 5 1 2>/dev/null || true
mknod -m 666 null c 1 3 2>/dev/null || true
mknod -m 666 zero c 1 5 2>/dev/null || true
mknod -m 666 tty c 5 0 2>/dev/null || true
mknod -m 666 random c 1 8 2>/dev/null || true
mknod -m 666 urandom c 1 9 2>/dev/null || true

# ============================================
# PACK INITRAMFS
# ============================================
echo "[5/5] Packing initramfs..."

cd "$INITRAMFS_DIR"
find . -print0 | cpio --null -ov --format=newc 2>/dev/null | gzip -9 > "$OUTPUT"

SIZE=$(du -h "$OUTPUT" | cut -f1)
echo ""
echo "Initramfs built: $OUTPUT ($SIZE)"
echo "Contents: $(find . -type f | wc -l) files"
