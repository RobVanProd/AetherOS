#!/usr/bin/env python3
"""
The Scout
Profiles physical machine hardware and generates machine_identity.json
compatible with the Architect's format.

Usage:
    sudo ./scout.py                    # Scan local machine
    sudo ./scout.py -o machine.json    # Save to file
    sudo ./scout.py --dump-acpi        # Include ACPI table dumps
"""

import subprocess
import json
import hashlib
import re
import sys
from pathlib import Path
from dataclasses import dataclass, asdict
from typing import Optional, List, Dict
import argparse


@dataclass
class DeviceInfo:
    """Generic device information"""
    vendor_id: Optional[str]
    device_id: Optional[str]
    subsystem_vendor: Optional[str]
    subsystem_device: Optional[str]
    device_class: Optional[str]
    description: str


@dataclass
class MachineProfile:
    """Physical machine hardware profile"""
    machine_id: str
    profile_type: str  # "detected"
    hostname: str

    # System info
    dmi_manufacturer: Optional[str]
    dmi_product: Optional[str]
    dmi_version: Optional[str]
    dmi_bios_vendor: Optional[str]
    dmi_bios_version: Optional[str]

    # CPU
    cpu_model: str
    cpu_cores: int
    cpu_threads: int

    # Memory
    memory_total_mb: int

    # Devices
    pci_devices: List[DeviceInfo]
    usb_devices: List[DeviceInfo]

    # Network interfaces
    network_interfaces: List[Dict[str, str]]

    # Storage devices
    block_devices: List[Dict[str, str]]

    # ACPI (optional)
    acpi_tables: Optional[Dict[str, str]]


def run_command(cmd: List[str], check=True) -> str:
    """Execute command and return output"""
    try:
        result = subprocess.run(
            cmd,
            capture_output=True,
            text=True,
            check=check
        )
        return result.stdout
    except subprocess.CalledProcessError as e:
        print(f"Warning: Command failed: {' '.join(cmd)}", file=sys.stderr)
        print(f"  Error: {e.stderr}", file=sys.stderr)
        return ""
    except FileNotFoundError:
        print(f"Warning: Command not found: {cmd[0]}", file=sys.stderr)
        return ""


def scan_pci_devices() -> List[DeviceInfo]:
    """Scan PCI devices using lspci"""
    devices = []

    # lspci -nn format: "00:00.0 Host bridge [0600]: Intel... [8086:1234]"
    output = run_command(["lspci", "-nn"], check=False)

    for line in output.strip().split('\n'):
        if not line:
            continue

        # Extract vendor:device IDs from brackets
        ids_match = re.search(r'\[([0-9a-f]{4}):([0-9a-f]{4})\]', line, re.IGNORECASE)
        class_match = re.search(r'\[([0-9a-f]{4})\]', line, re.IGNORECASE)

        if ids_match:
            vendor_id = ids_match.group(1)
            device_id = ids_match.group(2)

            # Extract description (everything before the IDs)
            desc_part = line.split('[')[0].strip()
            # Remove PCI address at start (00:00.0)
            desc = ' '.join(desc_part.split()[1:])

            devices.append(DeviceInfo(
                vendor_id=vendor_id,
                device_id=device_id,
                subsystem_vendor=None,
                subsystem_device=None,
                device_class=class_match.group(1) if class_match else None,
                description=desc
            ))

    return devices


def scan_usb_devices() -> List[DeviceInfo]:
    """Scan USB devices using lsusb"""
    devices = []

    # lsusb format: "Bus 001 Device 002: ID 046d:c52b Logitech, Inc. Unifying Receiver"
    output = run_command(["lsusb"], check=False)

    for line in output.strip().split('\n'):
        if not line:
            continue

        # Extract vendor:device IDs
        match = re.search(r'ID ([0-9a-f]{4}):([0-9a-f]{4})\s+(.+)', line, re.IGNORECASE)
        if match:
            vendor_id = match.group(1)
            device_id = match.group(2)
            description = match.group(3).strip()

            devices.append(DeviceInfo(
                vendor_id=vendor_id,
                device_id=device_id,
                subsystem_vendor=None,
                subsystem_device=None,
                device_class=None,
                description=description
            ))

    return devices


def get_dmi_info() -> Dict[str, Optional[str]]:
    """Extract DMI/SMBIOS information"""
    dmi = {}

    # Try dmidecode (requires root)
    for field, dmi_type in [
        ("manufacturer", "system-manufacturer"),
        ("product", "system-product-name"),
        ("version", "system-version"),
        ("bios_vendor", "bios-vendor"),
        ("bios_version", "bios-version"),
    ]:
        value = run_command(["dmidecode", "-s", dmi_type], check=False).strip()
        dmi[f"dmi_{field}"] = value if value else None

    return dmi


def get_cpu_info() -> Dict[str, any]:
    """Extract CPU information from /proc/cpuinfo"""
    cpu_info = {
        "cpu_model": "Unknown CPU",
        "cpu_cores": 1,
        "cpu_threads": 1,
    }

    try:
        with open("/proc/cpuinfo", "r") as f:
            content = f.read()

        # Extract model name
        model_match = re.search(r'model name\s*:\s*(.+)', content)
        if model_match:
            cpu_info["cpu_model"] = model_match.group(1).strip()

        # Count physical cores and threads
        physical_ids = set(re.findall(r'physical id\s*:\s*(\d+)', content))
        cpu_info["cpu_cores"] = len(physical_ids) if physical_ids else 1

        processors = len(re.findall(r'^processor\s*:', content, re.MULTILINE))
        cpu_info["cpu_threads"] = processors

    except FileNotFoundError:
        pass

    return cpu_info


def get_memory_info() -> int:
    """Get total system memory in MB"""
    try:
        with open("/proc/meminfo", "r") as f:
            for line in f:
                if line.startswith("MemTotal:"):
                    # MemTotal is in kB
                    kb = int(line.split()[1])
                    return kb // 1024
    except FileNotFoundError:
        pass

    return 0


def get_network_interfaces() -> List[Dict[str, str]]:
    """List network interfaces"""
    interfaces = []

    try:
        # Read from /sys/class/net
        net_dir = Path("/sys/class/net")
        for iface_path in net_dir.iterdir():
            if iface_path.name in ["lo"]:  # Skip loopback
                continue

            iface = {"name": iface_path.name}

            # Read MAC address
            try:
                mac_file = iface_path / "address"
                if mac_file.exists():
                    iface["mac"] = mac_file.read_text().strip()
            except:
                pass

            # Read driver info
            try:
                driver_link = iface_path / "device" / "driver"
                if driver_link.exists():
                    iface["driver"] = driver_link.resolve().name
            except:
                pass

            interfaces.append(iface)

    except FileNotFoundError:
        pass

    return interfaces


def get_block_devices() -> List[Dict[str, str]]:
    """List block devices"""
    devices = []

    output = run_command(["lsblk", "-ndo", "NAME,SIZE,TYPE,MODEL"], check=False)

    for line in output.strip().split('\n'):
        if not line:
            continue

        parts = line.split(maxsplit=3)
        if len(parts) >= 3 and parts[2] == "disk":
            device = {
                "name": parts[0],
                "size": parts[1] if len(parts) > 1 else "unknown",
                "model": parts[3].strip() if len(parts) > 3 else "unknown",
            }
            devices.append(device)

    return devices


def dump_acpi_tables() -> Optional[Dict[str, str]]:
    """Dump ACPI tables (requires root and acpidump)"""
    tables = {}

    # Try to dump DSDT
    dsdt = run_command(["acpidump", "-b", "-t", "DSDT"], check=False)
    if dsdt:
        tables["DSDT"] = dsdt[:500]  # Truncate for JSON

    # Try to dump SSDT
    ssdt = run_command(["acpidump", "-b", "-t", "SSDT"], check=False)
    if ssdt:
        tables["SSDT"] = ssdt[:500]  # Truncate for JSON

    return tables if tables else None


def generate_machine_id(profile: MachineProfile) -> str:
    """Generate unique machine ID from hardware fingerprint"""
    # Use DMI info + CPU + PCI devices for fingerprint
    fingerprint = {
        "dmi": (profile.dmi_manufacturer, profile.dmi_product, profile.dmi_version),
        "cpu": profile.cpu_model,
        "pci_devices": [(d.vendor_id, d.device_id) for d in profile.pci_devices[:10]],  # First 10
    }

    fingerprint_str = json.dumps(fingerprint, sort_keys=True)
    machine_id = hashlib.sha256(fingerprint_str.encode()).hexdigest()[:12]

    return machine_id


def scan_machine(include_acpi: bool = False) -> MachineProfile:
    """Scan local machine hardware"""

    print("The Scout: Scanning hardware...", file=sys.stderr)
    print("", file=sys.stderr)

    # Collect all information
    dmi_info = get_dmi_info()
    cpu_info = get_cpu_info()
    memory_mb = get_memory_info()

    print(f"System: {dmi_info.get('dmi_manufacturer', 'Unknown')} {dmi_info.get('dmi_product', 'Unknown')}", file=sys.stderr)
    print(f"CPU: {cpu_info['cpu_model']} ({cpu_info['cpu_threads']} threads)", file=sys.stderr)
    print(f"Memory: {memory_mb} MB", file=sys.stderr)
    print("", file=sys.stderr)

    print("Scanning PCI devices...", file=sys.stderr)
    pci_devices = scan_pci_devices()
    print(f"  Found {len(pci_devices)} PCI devices", file=sys.stderr)

    print("Scanning USB devices...", file=sys.stderr)
    usb_devices = scan_usb_devices()
    print(f"  Found {len(usb_devices)} USB devices", file=sys.stderr)

    print("Scanning network interfaces...", file=sys.stderr)
    network_interfaces = get_network_interfaces()
    print(f"  Found {len(network_interfaces)} network interfaces", file=sys.stderr)

    print("Scanning block devices...", file=sys.stderr)
    block_devices = get_block_devices()
    print(f"  Found {len(block_devices)} block devices", file=sys.stderr)

    acpi_tables = None
    if include_acpi:
        print("Dumping ACPI tables...", file=sys.stderr)
        acpi_tables = dump_acpi_tables()

    print("", file=sys.stderr)

    # Create profile
    hostname = run_command(["hostname"], check=False).strip() or "unknown"

    profile = MachineProfile(
        machine_id="",  # Will be generated
        profile_type="detected",
        hostname=hostname,
        **dmi_info,
        **cpu_info,
        memory_total_mb=memory_mb,
        pci_devices=pci_devices,
        usb_devices=usb_devices,
        network_interfaces=network_interfaces,
        block_devices=block_devices,
        acpi_tables=acpi_tables,
    )

    # Generate machine ID
    profile.machine_id = generate_machine_id(profile)

    return profile


def main():
    parser = argparse.ArgumentParser(
        description="The Scout - Physical machine hardware profiler"
    )
    parser.add_argument(
        "-o", "--output",
        type=Path,
        help="Output JSON file (default: stdout)"
    )
    parser.add_argument(
        "--dump-acpi",
        action="store_true",
        help="Include ACPI table dumps (requires root)"
    )
    parser.add_argument(
        "--pretty",
        action="store_true",
        help="Pretty-print JSON output"
    )

    args = parser.parse_args()

    # Check for root if needed
    if args.dump_acpi and subprocess.run(["id", "-u"], capture_output=True).stdout.strip() != b"0":
        print("Warning: ACPI dump requires root privileges", file=sys.stderr)

    # Scan machine
    profile = scan_machine(include_acpi=args.dump_acpi)

    # Convert to dict
    profile_dict = asdict(profile)

    # Output
    indent = 2 if args.pretty else None
    json_output = json.dumps(profile_dict, indent=indent)

    if args.output:
        args.output.write_text(json_output)
        print(f"Profile saved to: {args.output}", file=sys.stderr)
        print(f"Machine ID: {profile.machine_id}", file=sys.stderr)
    else:
        print(json_output)


if __name__ == "__main__":
    main()
