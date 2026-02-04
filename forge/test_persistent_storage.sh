#!/bin/bash
#
# Test Aether OS Persistent Storage
# Creates a virtual disk, installs Aether, and boots from it
#

set -e

echo "====================================="
echo "  AETHER PERSISTENT STORAGE TEST"
echo "====================================="
echo ""

DISK_IMAGE="/tmp/aether_disk.img"
DISK_SIZE="512M"

# Clean up old test
rm -f "$DISK_IMAGE"

# Step 1: Create virtual disk
echo "[1/5] Creating virtual disk ($DISK_SIZE)..."
qemu-img create -f raw "$DISK_IMAGE" "$DISK_SIZE" > /dev/null

# Step 2: Partition and format
echo "[2/5] Partitioning and formatting..."
(
echo n      # New partition
echo p      # Primary
echo 1      # Partition number
echo        # Default first sector
echo        # Default last sector
echo w      # Write changes
) | fdisk "$DISK_IMAGE" > /dev/null 2>&1

# Mount disk image as loop device
LOOP_DEV=$(sudo losetup -f --show -P "$DISK_IMAGE")
echo "  Loop device: $LOOP_DEV"

# Format partition
sudo mkfs.ext4 -F -L "AETHER_ROOT" "${LOOP_DEV}p1" > /dev/null 2>&1

# Step 3: Install Aether
echo "[3/5] Installing Aether OS..."
sudo mkdir -p /mnt/aether_test
sudo mount "${LOOP_DEV}p1" /mnt/aether_test

# Create structure
sudo mkdir -p /mnt/aether_test/{bin,sbin,etc,proc,sys,dev,tmp,run,var,root,boot,home}
sudo mkdir -p /mnt/aether_test/var/{log,tmp}

# Copy kernel
sudo cp build/arch/x86/boot/bzImage /mnt/aether_test/boot/vmlinuz

# Copy BusyBox
sudo cp /bin/busybox /mnt/aether_test/bin/ 2>/dev/null || {
    # Download busybox if not available
    wget -q https://busybox.net/downloads/binaries/1.35.0-x86_64-linux-musl/busybox -O /tmp/busybox
    sudo cp /tmp/busybox /mnt/aether_test/bin/busybox
}
sudo chmod +x /mnt/aether_test/bin/busybox

# Create symlinks
cd /mnt/aether_test/bin
for cmd in sh cat ls pwd echo ps top free df mount umount ip ping wget vi clear; do
    sudo ln -sf busybox "$cmd" 2>/dev/null
done
cd -

# Create init
sudo tee /mnt/aether_test/sbin/init > /dev/null << 'INIT'
#!/bin/sh
mount -t proc none /proc
mount -t sysfs none /sys
mount -t devtmpfs none /dev

hostname aether
ip link set lo up
ip addr add 127.0.0.1/8 dev lo

export HOME=/root PATH=/bin:/sbin TERM=linux
export PS1='\033[1;36maether\033[0m:\033[1;34m\w\033[0m\$ '

# Create a test file to prove persistence
if [ ! -f /root/boot_count ]; then
    echo "1" > /root/boot_count
else
    COUNT=$(cat /root/boot_count)
    echo $((COUNT + 1)) > /root/boot_count
fi

clear
echo ""
echo "  AETHER OS v0.3 - PERSISTENT MODE"
echo "  Boot count: $(cat /root/boot_count)"
echo ""
echo "  Files in /root persist across reboots!"
echo "  Try: echo 'hello' > /root/test.txt"
echo ""

cd /root
exec /bin/sh
INIT
sudo chmod +x /mnt/aether_test/sbin/init

# Create aether-release
echo "v0.3.0-persistent" | sudo tee /mnt/aether_test/etc/aether-release > /dev/null

# Unmount
sudo umount /mnt/aether_test
sudo losetup -d "$LOOP_DEV"

echo "[4/5] Installation complete!"
echo ""

# Step 4: Boot from persistent disk
echo "[5/5] Booting Aether from persistent disk..."
echo "  (Press Ctrl+C after verifying boot)"
echo ""
sleep 2

timeout 15 qemu-system-x86_64 \
    -kernel build/arch/x86/boot/bzImage \
    -append "root=/dev/sda1 console=ttyS0 rw" \
    -drive file="$DISK_IMAGE",format=raw \
    -nographic \
    -m 512 \
    -netdev user,id=net0 \
    -device e1000,netdev=net0 \
    2>&1 || true

echo ""
echo "====================================="
echo "  TEST COMPLETE"
echo "====================================="
echo ""
echo "Virtual disk created at: $DISK_IMAGE"
echo "To boot again: qemu-system-x86_64 -kernel build/arch/x86/boot/bzImage \\"
echo "  -append 'root=/dev/sda1 console=ttyS0 rw' \\"
echo "  -drive file=$DISK_IMAGE,format=raw -nographic"
echo ""
