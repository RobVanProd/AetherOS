#!/bin/sh
#
# Aether OS Installer
# Installs Aether to a target disk
#

echo ""
echo "====================================="
echo "  AETHER OS INSTALLER"
echo "====================================="
echo ""

# Check if running as root
if [ "$(id -u)" != "0" ]; then
    echo "Error: Must run as root"
    exit 1
fi

# Detect available disks
echo "Detecting available disks..."
echo ""
lsblk -ndo NAME,SIZE,TYPE | grep disk

echo ""
echo "Available disks:"
for disk in /dev/sd? /dev/vd? /dev/nvme?n?; do
    if [ -b "$disk" ]; then
        SIZE=$(lsblk -ndo SIZE "$disk")
        echo "  $disk ($SIZE)"
    fi
done

echo ""
read -p "Enter target disk (e.g., /dev/sda): " TARGET

if [ ! -b "$TARGET" ]; then
    echo "Error: $TARGET is not a valid block device"
    exit 1
fi

echo ""
echo "WARNING: This will ERASE ALL DATA on $TARGET!"
read -p "Are you sure? Type 'yes' to continue: " CONFIRM

if [ "$CONFIRM" != "yes" ]; then
    echo "Installation cancelled"
    exit 0
fi

echo ""
echo "Installing Aether OS to $TARGET..."

# ============================================
# Step 1: Partition the disk
# ============================================
echo "[1/6] Partitioning disk..."

# Create partition table
dd if=/dev/zero of="$TARGET" bs=1M count=10 2>/dev/null

# Simple MBR partition: one ext4 partition
(
echo n      # New partition
echo p      # Primary
echo 1      # Partition number
echo        # Default first sector
echo        # Default last sector
echo w      # Write changes
) | fdisk "$TARGET" 2>/dev/null

sleep 2

# Determine partition name
if echo "$TARGET" | grep -q nvme; then
    PART="${TARGET}p1"
else
    PART="${TARGET}1"
fi

# ============================================
# Step 2: Format partition
# ============================================
echo "[2/6] Formatting partition..."
mkfs.ext4 -F -L "AETHER_ROOT" "$PART" 2>/dev/null

# ============================================
# Step 3: Mount target
# ============================================
echo "[3/6] Mounting target..."
mkdir -p /mnt/target
mount "$PART" /mnt/target

# ============================================
# Step 4: Create directory structure
# ============================================
echo "[4/6] Creating filesystem structure..."
mkdir -p /mnt/target/{bin,sbin,etc,proc,sys,dev,tmp,run,var,root,usr/bin,usr/sbin,boot,home}
mkdir -p /mnt/target/var/{log,tmp}

# ============================================
# Step 5: Copy system files
# ============================================
echo "[5/6] Copying system files..."

# Copy kernel and initramfs
cp /boot/vmlinuz /mnt/target/boot/ 2>/dev/null || \
    cp /vmlinuz /mnt/target/boot/ 2>/dev/null || \
    echo "Warning: Kernel not copied"

cp /boot/initramfs.cpio.gz /mnt/target/boot/ 2>/dev/null || \
    echo "Warning: Initramfs not copied"

# Copy BusyBox and create symlinks
cp /bin/busybox /mnt/target/bin/
cd /mnt/target/bin
for cmd in sh ash cat cp mv rm mkdir rmdir ls ln chmod chown \
           grep sed awk cut head tail sort uniq wc tr \
           echo printf test true false sleep date hostname uname \
           mount umount ps top kill killall free df du \
           ping ip wget vi tar gzip gunzip clear dmesg mdev \
           poweroff reboot halt sysinfo help; do
    ln -sf busybox "$cmd" 2>/dev/null
done

cd /mnt/target/sbin
for cmd in init mdev ifconfig route ip halt poweroff reboot syslogd udhcpc; do
    ln -sf ../bin/busybox "$cmd" 2>/dev/null
done
cd /

# Copy init script
cat > /mnt/target/sbin/init-installed << 'INIT'
#!/bin/sh
# Aether installed system init

mount -t proc none /proc
mount -t sysfs none /sys
mount -t devtmpfs none /dev

hostname aether
ip link set lo up
ip addr add 127.0.0.1/8 dev lo

export HOME=/root PATH=/bin:/sbin:/usr/bin:/usr/sbin TERM=linux
export PS1='\033[1;36maether\033[0m:\033[1;34m\w\033[0m\$ '

clear
echo ""
echo "  AETHER OS v0.3 - installed mode"
echo "  Type 'help' for commands"
echo ""

cd /root
exec /bin/sh -l
INIT
chmod +x /mnt/target/sbin/init-installed

# ============================================
# Step 6: Create config files
# ============================================
echo "[6/6] Creating configuration..."

echo "root:x:0:0:root:/root:/bin/sh" > /mnt/target/etc/passwd
echo "root:x:0:" > /mnt/target/etc/group
echo "aether" > /mnt/target/etc/hostname
echo "v0.3.0" > /mnt/target/etc/aether-release

cat > /mnt/target/etc/fstab << 'FSTAB'
# Aether OS fstab
proc /proc proc defaults 0 0
sysfs /sys sysfs defaults 0 0
devtmpfs /dev devtmpfs defaults 0 0
tmpfs /tmp tmpfs defaults 0 0
tmpfs /run tmpfs defaults 0 0
FSTAB

# Create DHCP script
cat > /mnt/target/etc/udhcpc.script << 'DHCP'
#!/bin/sh
case "$1" in
    deconfig) ip addr flush dev $interface; ip link set $interface up;;
    bound|renew)
        ip addr add $ip/$mask dev $interface
        [ -n "$router" ] && ip route add default via $router dev $interface
        [ -n "$dns" ] && echo "nameserver $dns" > /etc/resolv.conf
        ;;
esac
DHCP
chmod +x /mnt/target/etc/udhcpc.script

# ============================================
# Cleanup
# ============================================
sync
umount /mnt/target

echo ""
echo "====================================="
echo "  Installation complete!"
echo "====================================="
echo ""
echo "Aether OS has been installed to $TARGET"
echo "Partition: $PART"
echo ""
echo "To boot, add this to your bootloader:"
echo "  root=$PART console=tty1"
echo ""
echo "Or use GRUB:"
echo "  linux /boot/vmlinuz root=$PART console=tty1"
echo "  initrd /boot/initramfs.cpio.gz"
echo ""
