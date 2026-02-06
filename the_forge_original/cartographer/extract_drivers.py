#!/usr/bin/env python3
"""
The Cartographer
Extracts driver ↔ device ID mappings from Linux kernel source.
"""

import os
import re
import json
from pathlib import Path
from dataclasses import dataclass, asdict
from typing import Optional

KERNEL_SRC = Path("/forge/kernel_src")
OUTPUT_DIR = Path("/forge/data")

@dataclass
class Driver:
    name: str
    path: str
    category: str
    pci_ids: list[tuple[str, str]]  # (vendor, device)
    usb_ids: list[tuple[str, str]]  # (vendor, product)
    complexity: int  # lines of code
    
def extract_pci_ids(content: str) -> list[tuple[str, str]]:
    """Extract PCI device IDs from MODULE_DEVICE_TABLE macro."""
    ids = []
    
    # Pattern: { PCI_DEVICE(0xVENDOR, 0xDEVICE) }
    pattern = r'PCI_DEVICE\s*\(\s*(0x[0-9a-fA-F]+)\s*,\s*(0x[0-9a-fA-F]+)\s*\)'
    for match in re.finditer(pattern, content):
        vendor = match.group(1).lower()
        device = match.group(2).lower()
        ids.append((vendor, device))
    
    # Pattern: { PCI_VDEVICE(VENDOR, 0xDEVICE) } - vendor from define
    pattern2 = r'PCI_VDEVICE\s*\(\s*(\w+)\s*,\s*(0x[0-9a-fA-F]+)\s*\)'
    for match in re.finditer(pattern2, content):
        # We'd need to resolve the vendor define, skip for now
        pass
        
    return ids

def extract_usb_ids(content: str) -> list[tuple[str, str]]:
    """Extract USB device IDs from MODULE_DEVICE_TABLE macro."""
    ids = []
    
    # Pattern: { USB_DEVICE(0xVENDOR, 0xPRODUCT) }
    pattern = r'USB_DEVICE\s*\(\s*(0x[0-9a-fA-F]+)\s*,\s*(0x[0-9a-fA-F]+)\s*\)'
    for match in re.finditer(pattern, content):
        vendor = match.group(1).lower()
        product = match.group(2).lower()
        ids.append((vendor, product))
    
    # Pattern: USB_DEVICE_ID with vendor/product fields
    pattern2 = r'\.idVendor\s*=\s*(0x[0-9a-fA-F]+).*?\.idProduct\s*=\s*(0x[0-9a-fA-F]+)'
    for match in re.finditer(pattern2, content, re.DOTALL):
        vendor = match.group(1).lower()
        product = match.group(2).lower()
        ids.append((vendor, product))
        
    return ids

def categorize_driver(path: str) -> str:
    """Determine driver category from path."""
    path_lower = path.lower()
    
    if '/gpu/' in path_lower or '/drm/' in path_lower:
        return 'gpu'
    elif '/net/' in path_lower:
        return 'network'
    elif '/usb/' in path_lower:
        return 'usb'
    elif '/input/' in path_lower:
        return 'input'
    elif '/sound/' in path_lower or '/audio/' in path_lower:
        return 'audio'
    elif '/block/' in path_lower or '/nvme/' in path_lower or '/ata/' in path_lower:
        return 'storage'
    elif '/pci/' in path_lower:
        return 'pci'
    elif '/acpi/' in path_lower:
        return 'acpi'
    else:
        return 'other'

def scan_drivers(kernel_path: Path) -> list[Driver]:
    """Scan kernel source for all drivers with device IDs."""
    drivers = []
    driver_dirs = [
        kernel_path / "drivers",
        kernel_path / "sound",
    ]
    
    for base_dir in driver_dirs:
        if not base_dir.exists():
            continue
            
        for c_file in base_dir.rglob("*.c"):
            try:
                content = c_file.read_text(errors='ignore')
            except Exception:
                continue
            
            # Only process files with device tables
            if 'MODULE_DEVICE_TABLE' not in content:
                continue
            
            pci_ids = extract_pci_ids(content)
            usb_ids = extract_usb_ids(content)
            
            if not pci_ids and not usb_ids:
                continue
            
            rel_path = str(c_file.relative_to(kernel_path))
            driver_name = c_file.stem
            
            driver = Driver(
                name=driver_name,
                path=rel_path,
                category=categorize_driver(rel_path),
                pci_ids=pci_ids,
                usb_ids=usb_ids,
                complexity=len(content.splitlines())
            )
            drivers.append(driver)
    
    return drivers

def main():
    print(f"Scanning kernel source: {KERNEL_SRC}")
    
    if not KERNEL_SRC.exists():
        print("ERROR: Kernel source not found!")
        print("Run inside Docker container with kernel source mounted.")
        return
    
    drivers = scan_drivers(KERNEL_SRC)
    
    # Summary stats
    total_pci = sum(len(d.pci_ids) for d in drivers)
    total_usb = sum(len(d.usb_ids) for d in drivers)
    
    print(f"Found {len(drivers)} drivers")
    print(f"  PCI device IDs: {total_pci}")
    print(f"  USB device IDs: {total_usb}")
    
    # Category breakdown
    categories = {}
    for d in drivers:
        categories[d.category] = categories.get(d.category, 0) + 1
    
    print("Categories:")
    for cat, count in sorted(categories.items(), key=lambda x: -x[1]):
        print(f"  {cat}: {count}")
    
    # Save manifest
    OUTPUT_DIR.mkdir(parents=True, exist_ok=True)
    manifest_path = OUTPUT_DIR / "driver_manifest.json"
    
    manifest = {
        "kernel_version": "6.6.70",
        "driver_count": len(drivers),
        "pci_id_count": total_pci,
        "usb_id_count": total_usb,
        "drivers": [asdict(d) for d in drivers]
    }
    
    with open(manifest_path, 'w') as f:
        json.dump(manifest, f, indent=2)
    
    print(f"\nManifest saved: {manifest_path}")

if __name__ == "__main__":
    main()
