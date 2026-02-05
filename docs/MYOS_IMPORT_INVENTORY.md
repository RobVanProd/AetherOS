# MyOS Import Inventory (into AetherOS)

This doc inventories what we should bring over from **MyOS** and where it should live in **AetherOS**, with an explicit plan for *how* we import it.

## Current state
- MyOS is already present in this repo under: `legacy/MyOS/` (history preserved via subtree).
- AetherOS canonical code layout is still being established; today we mostly boot/run legacy/MyOS.

## Import strategy (two lanes)

### Lane A — Keep MyOS as a history-preserving subtree (done/keep)
**Keep** `legacy/MyOS/` as a `git subtree` import.
- Purpose: provenance + bisectable history + reference implementation.
- Rule: do not “edit in place” for new architecture work; instead *adopt* modules into canonical paths.

### Lane B — Adopt modules into canonical AetherOS paths (copy/move, then evolve)
For each adopted module:
1) copy (or `git mv`) from `legacy/MyOS/...` into canonical paths
2) make it build in the new tree
3) add/refresh a minimal test or smoke demo
4) write a short design note if behavior/ABI changes

## What to import (and where it should live)

### 0) Boot + linker + early init
Source today:
- `legacy/MyOS/src/boot/multiboot.asm`
- `legacy/MyOS/src/kernel/kernel.ld` and/or `legacy/MyOS/src/kernel/linker.ld`
- `legacy/MyOS/src/kernel/kernel.c`

Target in AetherOS:
- `kernel/arch/x86/boot/` (multiboot + early entry)
- `kernel/arch/x86/linker/` (linker scripts)
- `kernel/init/` (kernel main/bringup)

Import method: **module copy** (Lane B)

### 1) CPU tables + interrupts
Source today:
- GDT/TSS: `legacy/MyOS/src/kernel/gdt.c`, `gdt_asm.asm`, `tss.c`
- IDT/interrupts: `legacy/MyOS/src/kernel/idt.c`, `interrupt.c`, `interrupt_asm.asm`
- PIC/PIT/timer: `legacy/MyOS/src/kernel/pic.c`, `timer.c`

Target:
- `kernel/arch/x86/` (gdt/idt/isr/tss/pic/pit)

Import method: **module copy**

### 2) Memory management
Source today:
- `legacy/MyOS/src/kernel/memory.c`, `mmap.c`, `paging.c`
- allocators: `heap.c`, `kheap.c`, `kmalloc.c`

Target:
- `kernel/mm/` (paging, physmem, kmalloc)
- `kernel/arch/x86/mm/` (paging impl details)

Import method: **module copy**

### 3) HAL + device discovery
Source today:
- `legacy/MyOS/src/kernel/hal.c`
- `legacy/MyOS/src/kernel/pci.c`
- `legacy/MyOS/src/kernel/driver.c`

Target:
- `kernel/hal/`
- `kernel/bus/pci/`
- `kernel/drivers/` (driver registry + probing)

Import method: **module copy**

### 4) Process / scheduling / signals
Source today:
- `legacy/MyOS/src/kernel/process.c`
- `legacy/MyOS/src/kernel/signal.c`
- `legacy/MyOS/src/kernel/test_process.c`

Target:
- `kernel/proc/` (process/tasking)
- `kernel/ipc/` (signals or future replacement)
- `tests/kernel/` (if we keep in-tree tests)

Import method: **module copy**

### 5) Filesystem + storage
Source today:
- kernel FS: `legacy/MyOS/src/kernel/fs.c`
- ATA driver: `legacy/MyOS/src/drivers/storage/ata.c`

Target:
- `kernel/fs/`
- `kernel/drivers/storage/ata/`

Import method: **module copy**

### 6) Networking
Source today:
- `legacy/MyOS/src/kernel/net/` and/or `legacy/MyOS/src/kernel/network.c`
- RTL8139: `legacy/MyOS/src/drivers/network/rtl8139.*`

Target:
- `kernel/net/`
- `kernel/drivers/net/rtl8139/`

Import method: **module copy**

### 7) Input, console, and UI (text/graphics/windowing)
Source today:
- input: `keyboard.c`, `mouse.c`
- terminal/shell: `terminal.c`, `shell.c`, `command.*`
- graphics/windowing: `graphics.c`, `window.c`, `cursor.c`, `font.c`

Target:
- `kernel/drivers/input/` (keyboard/mouse)
- `kernel/console/` (terminal, printk/logging, shell as debug tool)
- `kernel/ui/` (framebuffer/graphics/windowing) — or keep under `legacy` until V0 direction is decided

Import method: **module copy**, but consider deferring windowing until core kernel is stable.

### 8) Userland apps (if we keep them)
Source today:
- `legacy/MyOS/src/apps/*` (calculator, notepad, shell)

Target:
- `userspace/` (if we decide to keep a userspace app model)

Import method: **module copy** (optional / low priority)

### 9) Shared headers
Source today:
- `legacy/MyOS/src/include/**`
- `legacy/MyOS/src/kernel/include/**`

Target:
- `kernel/include/`
- `userspace/include/` (only if ABI is committed)

Import method: **module copy**, but prefer tightening/public-vs-private headers as we adopt.

## What NOT to import (or to remove from canonical history)
- `legacy/MyOS/nasm/**` (vendored NASM source/binary payload).
  - AetherOS repo rule is **source-only** and we should not carry toolchains in-tree.
  - Replace with toolchain setup docs + (optional) release artifacts, or a deterministic installer script.

## Concrete next steps (small PR-sized)
1) **Create canonical skeleton** directories:
   - `kernel/`, `kernel/arch/x86/`, `kernel/mm/`, `kernel/net/`, `kernel/fs/`, `kernel/drivers/`, `userspace/`
2) Adopt *one* module first (recommended: interrupts + GDT/IDT) and wire it into the current build.
3) Add a `tools/bootstrap_rust.sh` or doc section for installing Rust, so `make forge-test` works for new devs.
4) Decide on naming + boundaries:
   - `kernel/console/` vs `kernel/ui/` vs “keep UI in legacy until later”
5) Make a tracking issue/checklist for module adoption order (memory → proc → fs → net → drivers).
