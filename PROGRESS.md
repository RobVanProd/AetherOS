# AetherOS Development Progress

## Project Overview

**AetherOS** is an AI-native operating system by Aeternum Labs. It boots via QEMU VM (512MB RAM, Linux 6.6.70 kernel) and features a graphical framebuffer UI, AI-powered dashboard, and integrated world model.

---

## Completed Phases

### Phase 1–6: Foundation
- Bootable Linux kernel with custom init system
- Rust-based daemon architecture (aetherd, aurorad)
- CFC-JEPA world model integration (37M params)
- Brain server with Claude CLI + Ollama backend

### Phase 7–11: Nebula TUI Shell
- Serial console UI with ratatui 0.29 + crossterm 0.28
- Natural language interface via brain server
- System monitoring and AI query pipeline

### Phase 12: AI-Native OS
- Brain server integration (port 9200)
- Natural language command processing
- AI-assisted system management

### Phase 13: Proactive OS (8 sub-phases)
- **13a**: Feed infrastructure (FeedItem/FeedStore)
- **13b**: Proactive system monitor (TelemetryHistory, threshold alerts)
- **13c**: World model insights (CFC-JEPA polling, prediction error trends)
- **13d**: Background task system (TaskManager, `&` prefix for async)
- **13e**: Keyboard navigation (InputRouter, AppAction, panel focus)
- **13f**: Brain proactive endpoint (POST /v0/brain/proactive, 120s polling)
- **13g**: Rich widgets (mini_bar, sparkline, progress_bar)
- **13h**: Session context (/tmp/aether_session.json, topic extraction)

### Phase 14: Graphical Framebuffer UI (6 sub-phases)
- **14a**: Kernel DRM/FB/input config, QEMU `--graphical`, boot-gui/brain-demo-gui targets
- **14b**: nebula-fb crate with framebuffer rendering (tiny-skia 0.11, fontdue 0.9, Inter font)
- **14c**: Widget toolkit (card, status_bar, text_input, button, chart, progress) + Scene/Layout
- **14d**: Boot splash → Setup wizard (name + interests) → Generative dashboard (brain cards)
- **14e**: Brain /v0/brain/dashboard + Ollama backend (USE_LOCAL_MODEL env var)
- **14f**: Init prefers nebula-fb when /dev/fb0 exists, build pipeline includes it

### Phase 14.5: Audio, Mouse Cursor, and UX Polish
- **Mouse cursor**: Software cursor via evdev reader for USB-tablet absolute positioning
  - Scans `/dev/input/event*`, reads `EV_ABS` (ABS_X/ABS_Y scaled 0–32767 → screen coords)
  - `EV_KEY` for `BTN_LEFT` click detection
  - 12x18 white arrow cursor with dark outline rendered as overlay
  - Lazy retry for evdev devices (handles late USB enumeration)
  - Click hit-testing on setup wizard buttons/chips and dashboard omnibar
- **Audio system**: WAV playback via OSS `/dev/dsp` (primary) or ALSA `/dev/snd/pcmC0D0p` (fallback)
  - WAV header parser (chunk-based, handles fmt + data chunks)
  - OSS ioctls: `SNDCTL_DSP_SETFMT`, `SNDCTL_DSP_CHANNELS`, `SNDCTL_DSP_SPEED`
  - Threaded playback with atomic volume control for fade-out
  - `BOOT.wav` (744KB) embedded in binary via `include_bytes!`
  - `POST.wav` (7.5MB, converted to mono 22kHz) in initramfs at `/usr/share/sounds/post.wav`
  - Boot chime on splash, looping music during setup wizard, 3s fade-out at 80% completion
- **UX hints**: "Press any key to skip" on boot splash (1s fade-in), "Press Enter or Tab to continue" on setup Step 1
- **Kernel**: Intel HDA sound + OSS compat, USB UHCI/OHCI/EHCI/XHCI drivers
- **QEMU**: `qemu-xhci` USB controller (kernel-matched), SDL audiodev + intel-hda
- **FB optimization**: Faster RGBA→BGRA blit with dirty-frame skip
- **Build**: `make build-kernel` target for Docker kernel rebuilds

---

## Current Status

### What Works
- Serial console boot (`make boot`, `make brain-demo`) — fully functional
- Graphical boot (`make boot-gui`, `make brain-demo-gui`) — framebuffer UI at 1920x1080
- Boot splash with fade-in animation and skip hint
- Setup wizard with name input, interest chip selection, animated progress bar
- Generative dashboard with brain-powered cards, status bar, omnibar
- AI query pipeline via omnibar → brain server → response display
- Keyboard navigation throughout all scenes
- Software mouse cursor with click support
- Audio playback (boot chime + setup music) — **requires kernel rebuild** (see below)

### Known Issues / What Needs Work

#### Kernel Rebuilt (Feb 15 2026) — DONE
Kernel rebuilt with sound and full USB HCD support:
- Intel HDA audio → `/dev/dsp` (OSS) and `/dev/snd/pcmC0D0p` (ALSA)
- Full USB HCD support (UHCI + OHCI + EHCI + XHCI)
- OSS compatibility layer for simpler audio playback
- Kernel size: 4.4M (was 4.2M)

To rebuild again after config changes:
```bash
make build-kernel   # Runs Docker foundry, ~5-10 minutes
make build          # Rebuilds Rust + initramfs
```

#### Audio — Needs Runtime Verification
- Kernel now has sound drivers; QEMU has intel-hda + SDL audiodev
- `AudioPlayer` checks for `/dev/dsp` (OSS primary) or `/dev/snd/pcmC0D0p` (ALSA fallback)
- Boot chime plays during splash, POST music loops during setup wizard
- Needs testing with `make brain-demo-gui` to confirm audio works end-to-end

#### Mouse Cursor (Fixed in 14.5, needs QEMU + kernel match)
- QEMU now uses `qemu-xhci` USB controller (matches kernel's `CONFIG_USB_XHCI_HCD=y`)
- Previously used `-usb` (UHCI) which had no kernel driver
- Evdev reader has lazy retry — will find devices even if USB enumerates late

---

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│                    AetherOS VM (512MB)                    │
│                                                          │
│  ┌──────────────┐  ┌──────────────┐  ┌───────────────┐  │
│  │  nebula-fb   │  │   nebula     │  │   aetherd     │  │
│  │ (GUI shell)  │  │ (TUI shell)  │  │ (audit/policy)│  │
│  │  /dev/fb0    │  │  /dev/tty0   │  │  port 9101    │  │
│  └──────┬───────┘  └──────────────┘  └───────────────┘  │
│         │                                                │
│  ┌──────┴───────┐                    ┌───────────────┐  │
│  │  audio.rs    │                    │   aurorad     │  │
│  │  /dev/dsp    │                    │ (job router)  │  │
│  └──────────────┘                    │  port 9102    │  │
│                                      └───────┬───────┘  │
│                        QEMU SLIRP NAT        │          │
│                     guest: 10.0.2.15          │          │
└───────────────────────────┬───────────────────┘          │
                            │                              │
              ┌─────────────┴──────────────┐               │
              │        Host Machine         │              │
              │                             │              │
              │  ┌──────────┐ ┌──────────┐  │              │
              │  │  brain   │ │   cfcd   │  │              │
              │  │  server  │ │  server  │  │              │
              │  │ port 9200│ │ port 9100│  │              │
              │  └──────────┘ └──────────┘  │              │
              └─────────────────────────────┘              │
```

### Key Files

| Component | Path | Description |
|-----------|------|-------------|
| Nebula FB (GUI) | `forge/nebula-fb/src/` | Framebuffer shell — main.rs, scene.rs, audio.rs, input.rs |
| Nebula TUI | `forge/nebula-tui/src/` | Serial console shell |
| Brain Server | `forge/brain/brain_server.py` | AI backend (Claude + Ollama) |
| Aurorad | `forge/aurorad/src/main.rs` | Job router daemon |
| Aetherd | `forge/aetherd/src/main.rs` | Audit daemon |
| Kernel Config | `the_forge_original/foundry/build_kernel.sh` | Kernel build script |
| QEMU Boot | `tools/run_qemu.sh` | QEMU launch script |
| Initramfs | `tools/build_initramfs.sh` | Initramfs builder |
| Init System | `aether_init/init` | PID 1 init script |
| Sound Assets | `forge/nebula-fb/assets/boot.wav`, `assets/sounds/post.wav` | Audio files |

### Build Commands

| Command | Description |
|---------|-------------|
| `make build` | Build Rust binaries + initramfs |
| `make build-kernel` | Rebuild kernel in Docker (after config changes) |
| `make boot` | Build + boot serial console |
| `make boot-gui` | Build + boot graphical (1920x1080) |
| `make brain-demo` | Boot with brain server (serial) |
| `make brain-demo-gui` | Boot with brain server (graphical) |
| `make clean` | Remove build artifacts |

### Binary Sizes

| Binary | Size | Notes |
|--------|------|-------|
| nebula-fb | 2.7M | Includes embedded boot.wav (744KB) + Inter font |
| nebula-tui | 1.7M | Serial console UI |
| aurorad | 835K | Job router |
| aetherd | 651K | Audit daemon |
| busybox | 1.1M | Core utilities |
| Initramfs | 11M | All binaries + post.wav (7.5MB) |
| Kernel | 4.4M | Linux 6.6.70 (rebuilt Feb 15 with sound + USB) |

---

## Roadmap / Future Work

### Phase 15: Planned
- [ ] Window manager / multi-panel layout in framebuffer mode
- [ ] File browser scene with directory listing
- [ ] Settings scene for system configuration
- [ ] Notification system with toast popups
- [ ] Multi-touch / gesture support

### Phase 16: Planned
- [ ] Persistent storage (ext4 partition for user data)
- [ ] Package manager concept (installable AI skills)
- [ ] Multi-user support
- [ ] Network configuration UI

### Backlog
- [ ] Scroll support in dashboard response text
- [ ] Animated card transitions (slide-in)
- [ ] Theme customization (light mode option)
- [ ] Screenshot capture utility
- [ ] Performance profiling and optimization
- [ ] Automated CI/CD pipeline
- [ ] Integration tests for QEMU boot sequence

---

## Disk Space Notes

- `aurora_env` (785G) archived to 4TB drive at `/media/rob/2c6ec24e-.../aurora_env_backup.tar.gz` (657G compressed)
- Boot drive: ~30% used, 1.2T free
