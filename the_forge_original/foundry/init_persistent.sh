#!/bin/sh
#
# Aether OS Init System v0.3 - Persistent Storage Support
# Supports both live (tmpfs) and installed (persistent disk) modes
#

# ============================================
# PHASE 1: Essential mounts
# ============================================
mount -t proc none /proc
mount -t sysfs none /sys
mount -t devtmpfs none /dev 2>/dev/null || mknod -m 622 /dev/console c 5 1
mkdir -p /dev/pts /dev/shm
mount -t devpts devpts /dev/pts 2>/dev/null
mount -t tmpfs tmpfs /dev/shm 2>/dev/null

# ============================================
# PHASE 2: Detect boot mode
# ============================================
BOOT_MODE="live"
ROOT_DEV=""

# Check kernel command line for root device
if grep -q "root=" /proc/cmdline; then
    ROOT_DEV=$(cat /proc/cmdline | sed 's/.*root=\([^ ]*\).*/\1/')
    echo "[init] Found root device: $ROOT_DEV"
    BOOT_MODE="installed"
fi

# ============================================
# PHASE 3: Mount root filesystem
# ============================================
if [ "$BOOT_MODE" = "installed" ] && [ -n "$ROOT_DEV" ]; then
    echo "[init] Booting in INSTALLED mode"

    # Wait for device to appear
    for i in 1 2 3 4 5; do
        if [ -b "$ROOT_DEV" ]; then
            echo "[init] Device $ROOT_DEV found"
            break
        fi
        echo "[init] Waiting for $ROOT_DEV... ($i/5)"
        sleep 1
    done

    # Try to mount root
    if [ -b "$ROOT_DEV" ]; then
        mkdir -p /newroot
        if mount -t ext4 "$ROOT_DEV" /newroot 2>/dev/null; then
            echo "[init] Mounted $ROOT_DEV as root filesystem"

            # Check if it looks like a valid Aether installation
            if [ -f /newroot/etc/aether-release ]; then
                echo "[init] Valid Aether installation detected"

                # Move mounts to newroot
                mount --move /proc /newroot/proc
                mount --move /sys /newroot/sys
                mount --move /dev /newroot/dev

                # Switch to new root
                exec switch_root /newroot /sbin/init-installed
            else
                echo "[init] Not a valid Aether installation, falling back to live mode"
                umount /newroot
                BOOT_MODE="live"
            fi
        else
            echo "[init] Failed to mount $ROOT_DEV, falling back to live mode"
            BOOT_MODE="live"
        fi
    else
        echo "[init] Device $ROOT_DEV not found, falling back to live mode"
        BOOT_MODE="live"
    fi
fi

# ============================================
# PHASE 4: Live mode mounts
# ============================================
if [ "$BOOT_MODE" = "live" ]; then
    echo "[init] Booting in LIVE mode (tmpfs)"

    mount -t tmpfs tmpfs /tmp
    mount -t tmpfs tmpfs /run
    mount -t tmpfs tmpfs /var
    mkdir -p /var/log /var/tmp
fi

# ============================================
# PHASE 5: System configuration
# ============================================
hostname aether
echo "3 3 3 3" > /proc/sys/kernel/printk 2>/dev/null
echo /sbin/mdev > /proc/sys/kernel/hotplug 2>/dev/null
mdev -s 2>/dev/null

# ============================================
# PHASE 6: Networking
# ============================================
ip link set lo up 2>/dev/null
ip addr add 127.0.0.1/8 dev lo 2>/dev/null

# Try to bring up first ethernet interface
for iface in /sys/class/net/eth*; do
    if [ -d "$iface" ]; then
        IFNAME=$(basename "$iface")
        echo "[net] Configuring $IFNAME..."
        ip link set "$IFNAME" up 2>/dev/null

        # Try DHCP if available
        if command -v udhcpc > /dev/null; then
            udhcpc -i "$IFNAME" -s /etc/udhcpc.script -q -n -b 2>/dev/null &
        fi
        break
    fi
done

# ============================================
# PHASE 7: User environment
# ============================================
export HOME=/root
export PATH=/bin:/sbin:/usr/bin:/usr/sbin
export TERM=linux
export PS1='\033[1;36maether\033[0m:\033[1;34m\w\033[0m\$ '

mkdir -p /root

# Create helper utilities
cat > /bin/sysinfo << 'S'
#!/bin/sh
echo ""
echo "===== AETHER SYSTEM INFO ====="
echo "Boot Mode:  $BOOT_MODE"
echo "Kernel:     $(uname -r)"
echo "Uptime:     $(cut -d. -f1 /proc/uptime)s"
echo "CPU:        $(grep -c processor /proc/cpuinfo) cores"
echo "Memory:     $(awk '/MemTotal/{t=$2}/MemFree/{f=$2}END{printf "%.0f/%.0fMB",f/1024,t/1024}' /proc/meminfo)"
echo ""
echo "Network:"
ip -br addr 2>/dev/null | grep -v "^lo" || echo "  (no network)"
echo ""
echo "Storage:"
df -h 2>/dev/null | grep -E "^/dev|Filesystem" | head -5
echo "=============================="
S
chmod +x /bin/sysinfo

cat > /bin/help << 'H'
#!/bin/sh
echo ""
echo "===== AETHER OS COMMANDS ====="
echo "System:  sysinfo|ps|top|free|df|dmesg|poweroff"
echo "Network: ip addr|ping|wget"
echo "Files:   ls|cat|cp|mv|rm|mkdir|vi"
echo "Install: aether-install (install to disk)"
echo "=============================="
H
chmod +x /bin/help

# ============================================
# PHASE 8: Boot banner
# ============================================
echo "BOOT_SUCCESS" > /dev/console
clear 2>/dev/null || true
echo ""
echo "  ___       __  __             ____  _____"
echo " /   | ____/ /_/ /_  ___  ____/ __ \/ ___/"
echo "/ /| |/ _ \ __/ __ \/ _ \/ __/ / / /\__ \ "
echo "/ ___ /  __/ /_/ / / /  __/ / / /_/ /___/ /"
echo "/_/  |_\___/\__/_/ /_/\___/_/  \____//____/"
echo ""
echo "         v0.3 - $BOOT_MODE mode"
echo ""
echo "Type 'help' for commands"
if [ "$BOOT_MODE" = "live" ]; then
    echo "Type 'aether-install' to install to disk"
fi
echo ""

# ============================================
# PHASE 9: Drop to shell
# ============================================
cd /root
exec /bin/sh -l
