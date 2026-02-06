# Aether OS: Path to Usable Prototype

## Current State (v0.1 - "It Boots")
- ✅ Kernel compiles and boots in <1 second
- ✅ 120 synthetic machine configs
- ✅ 100% boot success rate in Crucible
- ✅ Bespoke kernel generation per machine
- ✅ Scout can profile real hardware
- ❌ No interactive shell (boots → halts)
- ❌ No networking
- ❌ No persistent storage
- ❌ No graphics

---

## Phase A: Interactive Console (v0.2)
**Goal:** Boot into a shell where you can actually do things.

### Components
- [x] Enhanced init system (init v0.2)
- [x] Full BusyBox symlinks (50+ commands)
- [x] System info utilities (sysinfo, help)
- [x] Writable tmpfs for /tmp, /var, /run
- [x] Basic /etc (passwd, hosts, hostname)
- [ ] Integrate into Foundry build pipeline
- [ ] Test on all 120 machine configs

### Validation
```bash
# After boot, you should be able to:
$ help              # See available commands
$ sysinfo           # CPU, memory, network info
$ ls /              # Browse filesystem
$ vi /tmp/test.txt  # Edit a file
$ ps                # See processes
$ dmesg | tail      # Kernel messages
```

---

## Phase B: Networking (v0.3)
**Goal:** Connect to the internet, download things.

### Components
- [ ] DHCP client working (udhcpc)
- [ ] DNS resolution (/etc/resolv.conf)
- [ ] wget/curl functional
- [ ] ping works to external hosts
- [ ] SSH server (dropbear - tiny SSH)
- [ ] NTP time sync (optional)

### Kernel Config Additions
```
CONFIG_NET=y
CONFIG_INET=y
CONFIG_IP_PNP=y
CONFIG_IP_PNP_DHCP=y
CONFIG_PACKET=y
CONFIG_UNIX=y

# Drivers (vary per machine)
CONFIG_E1000=y
CONFIG_VIRTIO_NET=y
CONFIG_R8169=y  # Realtek
CONFIG_IGB=y    # Intel
```

### Validation
```bash
$ ip addr                    # See IP from DHCP
$ ping 8.8.8.8               # Test connectivity
$ wget example.com           # Download
$ ssh user@aether            # Remote access (from host)
```

---

## Phase C: Persistent Storage (v0.4)
**Goal:** Files survive reboot.

### Components
- [ ] Detect and mount root filesystem
- [ ] ext4 support in kernel
- [ ] Partition detection
- [ ] Install to disk script
- [ ] Bootloader (GRUB or direct EFI stub)

### Modes
1. **Live mode** - Boot from USB, tmpfs root
2. **Installed mode** - Boot from disk, persistent root

### Filesystem Layout
```
/
├── bin/        # BusyBox + core utilities
├── etc/        # Configuration
├── home/       # User data (persistent)
├── root/       # Root home (persistent)
├── tmp/        # Temporary (tmpfs)
├── var/        # Variable data (persistent)
└── usr/        # Additional software
```

---

## Phase D: Package Management (v0.5)
**Goal:** Install additional software.

### Options (pick one)
1. **Static binaries** - Just wget pre-compiled binaries
2. **apk (Alpine)** - Lightweight, huge package repo
3. **Custom** - Simple tar-based packages

### Recommended: Hybrid
- BusyBox core (built-in)
- apk for additional packages
- Pull from Alpine repos (compatible)

```bash
$ apk add python3
$ apk add git
$ apk add neovim
```

---

## Phase E: Graphics (v0.6)
**Goal:** Visual interface, not just text.

### Layers
1. **Framebuffer** - Direct pixel access, simplest
2. **DRM/KMS** - Modern, hardware accelerated
3. **Wayland** - Full compositor (cage, sway)
4. **Nebula** - Aether's custom shell

### Minimal Graphics Stack
```
Kernel DRM → wlroots → cage (single-app compositor) → Nebula
```

### For prototype, framebuffer is enough:
- Console with colors
- Maybe a simple TUI framework (ncurses)
- Nebula can be text-based initially

---

## Phase F: Nebula Shell (v0.7)
**Goal:** The "Intent-Centric" interface begins.

### Components
- [ ] Omni-bar (command/search interface)
- [ ] Context-aware suggestions
- [ ] Semantic file access (vectors later)
- [ ] Facet loading (WASM modules)

### Initial Implementation
Even text-based, Omni-bar can work:
```
┌─────────────────────────────────────────────┐
│ > open my notes from yesterday              │
├─────────────────────────────────────────────┤
│ 📄 notes-2026-01-02.md (modified 4:32pm)    │
│ 📄 meeting-notes.md (modified 2:15pm)       │
│ 📝 Create new note...                       │
└─────────────────────────────────────────────┘
```

This is where Aurora could plug in - understanding intent.

---

## Timeline Estimate

| Phase | Target | Effort |
|-------|--------|--------|
| A - Interactive | This week | 2-3 days |
| B - Networking | Next week | 3-4 days |
| C - Storage | Week 3 | 4-5 days |
| D - Packages | Week 4 | 2-3 days |
| E - Graphics | Month 2 | 1-2 weeks |
| F - Nebula | Month 2-3 | Ongoing |

---

## Next Immediate Steps

1. **Integrate init v0.2 into Foundry**
   - Replace current minimal init
   - Run Crucible on all 120 machines
   - Verify shell prompt reached

2. **Boot on The Forge (real hardware)**
   - Scout already profiled your Ryzen 9
   - Generate bespoke kernel for it
   - Boot from USB on actual hardware

3. **Add networking to kernel config**
   - Enable e1000, virtio-net, r8169
   - Test DHCP in QEMU
   - First `wget` from inside Aether

---

## The Vision

```
Boot → Nebula greets you → "What would you like to do?"
     → You type natural language
     → System understands intent
     → Facets compose to accomplish task
     → Context persists across sessions
```

We're building toward that. Each phase gets us closer.
