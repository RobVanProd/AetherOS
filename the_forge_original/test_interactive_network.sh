#!/bin/bash
# Interactive network test for Aether OS

echo "Booting Aether with network, sending test commands..."

# Boot and send commands
(
sleep 4  # Wait for boot
echo "ip addr show"
sleep 1
echo "ip route"
sleep 1
echo "ping -c 2 8.8.8.8"
sleep 3
echo "wget -O /tmp/test http://example.com 2>&1 | head -5"
sleep 3
echo "poweroff"
) | sg docker -c "docker run --rm -i --entrypoint /bin/bash -v $(pwd):/forge aether-os-builder -c '
qemu-system-x86_64 \
    -kernel /forge/build/arch/x86/boot/bzImage \
    -initrd /forge/images/initramfs.cpio.gz \
    -nographic \
    -append \"console=ttyS0\" \
    -m 512 \
    -netdev user,id=net0 \
    -device e1000,netdev=net0 \
    2>&1
'" 2>&1 | tee /tmp/aether_net_test.log

echo ""
echo "=== Network Test Results ==="
grep -E "(inet |route|ping|wget|bytes)" /tmp/aether_net_test.log | tail -15
