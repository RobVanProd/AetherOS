#!/usr/bin/env python3
"""
The Architect
Generates synthetic machine configurations for boot testing.
"""

import json
import random
import hashlib
from pathlib import Path
from dataclasses import dataclass, asdict
from typing import Optional

import os
DATA_DIR = Path(os.getenv("DATA_DIR", "/forge/data"))
MACHINES_DIR = Path(os.getenv("MACHINES_DIR", "/forge/machines"))

@dataclass
class MachineIdentity:
    machine_id: str
    profile: str  # "minimal", "desktop", "server", "workstation", "embedded"
    cpu: dict
    memory_mb: int
    storage: dict
    network: Optional[dict]
    usb_controller: Optional[str]
    usb_devices: list[dict]
    gpu: Optional[dict]
    sound: Optional[dict]
    qemu_args: list[str]

# QEMU-supported virtual hardware that we know works
QEMU_HARDWARE = {
    "cpu": [
        {"model": "qemu64", "cores": 1},
        {"model": "qemu64", "cores": 2},
        {"model": "qemu64", "cores": 4},
        {"model": "qemu64", "cores": 8},
        {"model": "qemu64", "cores": 16},
        {"model": "qemu64", "cores": 32},
        {"model": "Nehalem", "cores": 4},
        {"model": "SandyBridge", "cores": 2},
        {"model": "IvyBridge", "cores": 4},
        {"model": "Haswell", "cores": 4},
        {"model": "Broadwell", "cores": 8},
        {"model": "Skylake-Client", "cores": 4},
    ],
    "memory_mb": [128, 256, 512, 768, 1024, 1536, 2048, 3072, 4096, 8192, 16384, 32768, 65536, 131072],
    "storage": [
        {"type": "virtio-blk", "size_gb": 1, "bus": "virtio"},
        {"type": "ide-hd", "size_gb": 1, "bus": "ide", "model": "QEMU HARDDISK"},
        {"type": "ide-cd", "size_gb": 0, "bus": "ide", "model": "QEMU DVD-ROM"},
        {"type": "nvme", "size_gb": 1, "bus": "pcie", "model": "QEMU NVMe"},
        {"type": "scsi-hd", "size_gb": 1, "bus": "scsi", "model": "QEMU SCSI"},
        {"type": "usb-storage", "size_gb": 1, "bus": "usb", "model": "QEMU USB HARDDRIVE"},
    ],
    "network": [
        {"type": "virtio-net-pci", "model": "virtio", "speed": "10000"},
        {"type": "e1000", "model": "e1000", "speed": "1000"},
        {"type": "e1000e", "model": "e1000e", "speed": "1000"},
        {"type": "rtl8139", "model": "rtl8139", "speed": "100"},
        {"type": "ne2k_pci", "model": "ne2k_pci", "speed": "10"},
        {"type": "pcnet", "model": "pcnet", "speed": "10"},
        {"type": "vmxnet3", "model": "vmxnet3", "speed": "10000"},
        None,  # No network
    ],
    "usb_devices": [
        {"type": "usb-kbd", "name": "QEMU USB Keyboard"},
        {"type": "usb-mouse", "name": "QEMU USB Mouse"},
        {"type": "usb-tablet", "name": "QEMU USB Tablet"},
    ],
    "usb_controller": [
        "piix3-usb-uhci",  # USB 1.1
        "usb-ehci",        # USB 2.0
        "qemu-xhci",       # USB 3.0
        None,              # No USB
    ],
    "gpu": [
        {"type": "VGA", "model": "std", "vram_mb": 16},
        {"type": "VGA", "model": "cirrus", "vram_mb": 4},
        {"type": "VGA", "model": "qxl", "vram_mb": 16},
        {"type": "VGA", "model": "virtio-vga", "vram_mb": 256},
        {"type": "VGA", "model": "vmware-svga", "vram_mb": 16},
        None,  # No GPU (headless)
    ],
    "sound": [
        {"type": "AC97", "model": "ac97"},
        {"type": "ES1370", "model": "es1370"},
        {"type": "Intel HDA", "model": "intel-hda"},
        {"type": "SB16", "model": "sb16"},
        None,  # No sound
    ],
}

PROFILES = {
    "minimal": {
        "description": "Bare minimum to boot",
        "memory_range": (128, 512),
        "cpu_cores": (1, 2),
        "usb_prob": 0.3,
        "usb_count": (0, 1),
        "network_prob": 0.0,
        "gpu_prob": 0.0,
        "sound_prob": 0.0,
    },
    "embedded": {
        "description": "IoT/embedded device",
        "memory_range": (128, 768),
        "cpu_cores": (1, 2),
        "usb_prob": 0.5,
        "usb_count": (0, 1),
        "network_prob": 0.7,
        "gpu_prob": 0.1,
        "sound_prob": 0.0,
    },
    "desktop": {
        "description": "Typical desktop config",
        "memory_range": (1024, 4096),
        "cpu_cores": (2, 8),
        "usb_prob": 0.95,
        "usb_count": (2, 3),
        "network_prob": 0.9,
        "gpu_prob": 0.8,
        "sound_prob": 0.7,
    },
    "workstation": {
        "description": "High-end workstation",
        "memory_range": (8192, 65536),
        "cpu_cores": (8, 32),
        "usb_prob": 1.0,
        "usb_count": (2, 3),
        "network_prob": 1.0,
        "gpu_prob": 1.0,
        "sound_prob": 0.5,
    },
    "server": {
        "description": "Server-like config",
        "memory_range": (4096, 131072),
        "cpu_cores": (4, 32),
        "usb_prob": 0.2,
        "usb_count": (0, 1),
        "network_prob": 1.0,
        "gpu_prob": 0.1,
        "sound_prob": 0.0,
    },
}

def generate_machine(profile_name: str = "minimal") -> MachineIdentity:
    """Generate a synthetic machine configuration."""
    profile = PROFILES[profile_name]

    # CPU - pick from matching core count range
    cpu_options = [c for c in QEMU_HARDWARE["cpu"]
                   if profile["cpu_cores"][0] <= c["cores"] <= profile["cpu_cores"][1]]
    cpu = random.choice(cpu_options) if cpu_options else {"model": "qemu64", "cores": profile["cpu_cores"][0]}

    # Memory
    memory = random.choice([m for m in QEMU_HARDWARE["memory_mb"]
                           if profile["memory_range"][0] <= m <= profile["memory_range"][1]])

    # Storage (always need one)
    storage = random.choice(QEMU_HARDWARE["storage"])

    # Network (probabilistic)
    network = None
    if random.random() < profile["network_prob"]:
        network = random.choice([n for n in QEMU_HARDWARE["network"] if n])

    # USB controller and devices
    usb_controller = None
    usb_devices = []
    if random.random() < profile["usb_prob"]:
        usb_controller = random.choice([u for u in QEMU_HARDWARE["usb_controller"] if u])
        usb_count = random.randint(*profile["usb_count"])
        usb_devices = random.sample(QEMU_HARDWARE["usb_devices"], min(usb_count, len(QEMU_HARDWARE["usb_devices"])))

    # GPU (probabilistic)
    gpu = None
    if random.random() < profile["gpu_prob"]:
        gpu = random.choice([g for g in QEMU_HARDWARE["gpu"] if g])

    # Sound (probabilistic)
    sound = None
    if random.random() < profile["sound_prob"]:
        sound = random.choice([s for s in QEMU_HARDWARE["sound"] if s])

    # Generate QEMU command line args
    qemu_args = build_qemu_args(cpu, memory, storage, network, usb_controller, usb_devices, gpu, sound)

    # Generate unique machine ID
    config_str = json.dumps([cpu, memory, storage, network, usb_controller, usb_devices, gpu, sound], sort_keys=True)
    machine_id = hashlib.sha256(config_str.encode()).hexdigest()[:12]

    return MachineIdentity(
        machine_id=machine_id,
        profile=profile_name,
        cpu=cpu,
        memory_mb=memory,
        storage=storage,
        network=network,
        usb_controller=usb_controller,
        usb_devices=usb_devices,
        gpu=gpu,
        sound=sound,
        qemu_args=qemu_args
    )

def build_qemu_args(cpu, memory, storage, network, usb_controller, usb_devices, gpu, sound) -> list[str]:
    """Build QEMU command line arguments for this machine."""
    args = [
        "-cpu", cpu["model"],
        "-smp", str(cpu["cores"]),
        "-m", str(memory),
        "-nographic",
        "-serial", "stdio",
    ]

    # Storage
    if storage["type"] == "virtio-blk":
        args.extend(["-drive", "file=DISK_IMAGE,format=raw,if=virtio"])
    elif storage["type"] == "ide-hd":
        args.extend(["-drive", "file=DISK_IMAGE,format=raw,if=ide,media=disk"])
    elif storage["type"] == "ide-cd":
        args.extend(["-drive", "file=DISK_IMAGE,format=raw,if=ide,media=cdrom"])
    elif storage["type"] == "nvme":
        args.extend(["-drive", "file=DISK_IMAGE,format=raw,if=none,id=nvme0"])
        args.extend(["-device", "nvme,serial=deadbeef,drive=nvme0"])
    elif storage["type"] == "scsi-hd":
        args.extend(["-device", "virtio-scsi-pci,id=scsi0"])
        args.extend(["-drive", "file=DISK_IMAGE,format=raw,if=none,id=hd0"])
        args.extend(["-device", "scsi-hd,drive=hd0,bus=scsi0.0"])
    elif storage["type"] == "usb-storage":
        args.extend(["-drive", "file=DISK_IMAGE,format=raw,if=none,id=usbdisk"])
        args.extend(["-device", "usb-storage,drive=usbdisk"])

    # Network
    if network:
        args.extend(["-netdev", "user,id=net0"])
        args.extend(["-device", f"{network['type']},netdev=net0"])
    else:
        args.extend(["-nic", "none"])

    # USB controller and devices
    if usb_controller:
        args.extend(["-device", usb_controller])
        for dev in usb_devices:
            args.extend(["-device", dev["type"]])

    # GPU
    if gpu:
        args.extend(["-vga", gpu["model"]])
    else:
        args.extend(["-vga", "none"])

    # Sound
    if sound:
        args.extend(["-device", sound["model"]])

    return args

def main():
    import sys

    # Parse count argument (default 8 for quick tests, 100+ for full runs)
    count_arg = int(sys.argv[1]) if len(sys.argv) > 1 else 8

    print(f"The Architect: Generating {count_arg} synthetic machines")

    MACHINES_DIR.mkdir(parents=True, exist_ok=True)

    # Distribution of profiles
    if count_arg <= 10:
        # Small run: test each profile
        machines_to_generate = [
            ("minimal", max(1, count_arg // 5)),
            ("embedded", max(1, count_arg // 5)),
            ("desktop", max(1, count_arg // 5)),
            ("workstation", max(1, count_arg // 10)),
            ("server", max(1, count_arg // 10)),
        ]
    else:
        # Large run: weighted distribution
        machines_to_generate = [
            ("minimal", int(count_arg * 0.20)),      # 20%
            ("embedded", int(count_arg * 0.15)),     # 15%
            ("desktop", int(count_arg * 0.40)),      # 40%
            ("workstation", int(count_arg * 0.15)),  # 15%
            ("server", int(count_arg * 0.10)),       # 10%
        ]

    all_machines = []
    seen_ids = set()

    for profile_name, count in machines_to_generate:
        print(f"\nGenerating {count} '{profile_name}' machines:")
        generated = 0
        attempts = 0
        max_attempts = count * 10  # Prevent infinite loops

        while generated < count and attempts < max_attempts:
            attempts += 1
            machine = generate_machine(profile_name)

            # Skip duplicates (hash collision)
            if machine.machine_id in seen_ids:
                continue

            seen_ids.add(machine.machine_id)
            all_machines.append(machine)
            generated += 1

            # Save individual machine file
            machine_path = MACHINES_DIR / f"{machine.machine_id}.json"
            with open(machine_path, 'w') as f:
                json.dump(asdict(machine), f, indent=2)

            # Summary output
            hw_summary = f"{machine.cpu['cores']}C/{memory_str(machine.memory_mb)}"
            hw_summary += f"/{storage.get('type', 'unknown')}" if (storage := machine.storage) else ""
            hw_summary += f"/{network.get('type', 'no-net')[:8]}" if (network := machine.network) else "/no-net"
            print(f"  {machine.machine_id}: {hw_summary}")

    # Save index
    index_path = MACHINES_DIR / "index.json"
    with open(index_path, 'w') as f:
        json.dump({
            "count": len(all_machines),
            "machines": [m.machine_id for m in all_machines],
            "profiles": {
                profile: len([m for m in all_machines if m.profile == profile])
                for profile in PROFILES.keys()
            }
        }, f, indent=2)

    print(f"\nGenerated {len(all_machines)} unique machines in {MACHINES_DIR}")
    print(f"Profile distribution: {dict(json.load(open(index_path))['profiles'])}")

def memory_str(mb: int) -> str:
    """Format memory size for display."""
    if mb >= 1024:
        return f"{mb//1024}G"
    return f"{mb}M"

if __name__ == "__main__":
    main()
