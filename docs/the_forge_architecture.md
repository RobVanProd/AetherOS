# The Forge: Architecture & Implementation Plan
**Aeternum Labs — Project Vitruvian**  
**Version:** 0.1 (Draft)  
**Date:** January 3, 2026

---

## 1. What Is The Forge?

The Forge is an automated system that:
1. Understands the space of possible hardware configurations
2. Transpiles Linux C drivers → Rust drivers
3. Compiles bespoke Aether OS kernels for specific machines
4. Validates those kernels via automated boot testing
5. Learns from success/failure to improve transpilation

It is both **build infrastructure** and **self-improving AI system**.

---

## 2. Core Subsystems

### 2.1 The Cartographer (Kernel Metadata Extraction)

**Purpose:** Map the entire Linux driver ecosystem into structured data.

**Inputs:**
- Linux kernel source tree (git clone, specific version)
- Device ID databases (pci.ids, usb.ids)

**Outputs:**
- `driver_manifest.json` — every driver with:
  - Supported device IDs (PCI, USB, ACPI)
  - Dependencies (other kernel modules)
  - Complexity score (lines of code, unsafe patterns)
  - Hardware category (GPU, NIC, storage, input, etc.)

**Implementation:**
```
Language: Python (parsing) + Rust (validation)
Approach:
  1. Parse Kconfig files for dependency trees
  2. Extract MODULE_DEVICE_TABLE macros from C source
  3. Cross-reference with pci.ids/usb.ids for human names
  4. Score complexity via static analysis (cyclomatic, unsafe patterns)
```

**Open Questions:**
- Do we version-lock to a specific kernel (e.g., 6.6 LTS) or track mainline?
- How do we handle out-of-tree drivers?

---

### 2.2 The Architect (Synthetic Machine Generator)

**Purpose:** Generate realistic virtual hardware configurations for training/testing.

**Inputs:**
- `driver_manifest.json`
- Hardware compatibility rules (what devices coexist)
- Real-world machine profiles (optional, for realism)

**Outputs:**
- `synthetic_machines/` — directory of `machine_identity.json` files representing plausible hardware combos

**Implementation:**
```
Approach:
  1. Define hardware "slots": CPU, GPU, NIC, Storage, USB controllers, etc.
  2. For each slot, sample from compatible drivers
  3. Apply constraints:
     - No two GPUs from different vendors (usually)
     - Laptop vs desktop power management profiles
     - Chipset-specific ACPI tables
  4. Generate QEMU command-line flags that emulate this config
```

**Realism Sources:**
- Scrape PCPartPicker builds for real-world combos
- Parse Linux Hardware Database (linux-hardware.org) submissions
- Start simple (commodity hardware), expand to edge cases

---

### 2.3 The Alchemist (Transpilation Agent)

**Purpose:** Convert Linux C drivers to Rust, learning patterns over time.

**This is the AI core.**

**Inputs:**
- C driver source code
- Hardware spec sheets (datasheets, register maps)
- Target device's ACPI quirks (from machine identity)
- Existing Rust driver examples (embassy, smoltcp, linux-rust drivers)

**Outputs:**
- Rust driver source
- Confidence score (safe vs unsafe ratio)
- Audit trail (which patterns were applied)

**Architecture Options:**

| Approach | Pros | Cons |
|----------|------|------|
| **Fine-tuned LLM** | Learns idioms, handles ambiguity | Expensive to train, may hallucinate |
| **Rule-based transpiler + LLM fallback** | Predictable for known patterns | Brittle on novel code |
| **Agent loop (Aurora?)** | Can reason, search docs, iterate | Slower, needs good tooling |

**Recommended:** Hybrid approach
1. **Pattern matcher** handles known idioms (80% of code)
   - `kmalloc/kfree` → Rust allocator
   - `spin_lock` → `Mutex`
   - `readl/writel` → volatile MMIO abstractions
2. **LLM agent** handles the rest
   - Ingests datasheet context
   - Reasons about ownership
   - Flags genuinely unsafe sections for shim wrapping

**The Skill System:**
```
skills/
  ├── usb_hid.skill        # Learned: USB HID devices follow this template
  ├── pci_init.skill       # Learned: PCI probe sequences map to this Rust
  ├── interrupt_handler.skill
  └── dma_buffer.skill

Each "skill" is a learned pattern:
  - Input signature (C AST pattern)
  - Output template (Rust code)
  - Confidence (validated boot count)
  - Edge cases (known failure modes)
```

Skills accumulate. The Forge gets better over time.

---

### 2.4 The Foundry (Kernel Compiler)

**Purpose:** Assemble transpiled drivers + seL4-Rust base into bootable image.

**Inputs:**
- `machine_identity.json`
- Transpiled Rust drivers
- seL4-Rust kernel base
- Aether userspace (Nebula shell, etc.)

**Outputs:**
- `aether-{machine_hash}.iso` — bootable image

**Implementation:**
```
Toolchain:
  - Rust nightly (for kernel features)
  - LLVM for cross-compilation
  - Custom linker script for seL4 memory layout

Build steps:
  1. Select drivers for target hardware
  2. Compile drivers as static libs
  3. Link into kernel binary
  4. Package with bootloader (UEFI stub)
  5. Generate ISO with EFI partition
```

**Phase 1 shortcut:** Use Linux kernel as base instead of seL4, swap later.

---

### 2.5 The Crucible (Automated Boot Testing)

**Purpose:** Validate compiled images actually boot.

**Inputs:**
- `aether-{machine_hash}.iso`
- Corresponding QEMU hardware config

**Outputs:**
- Boot log
- Success/failure classification
- Failure analysis (where did it die?)

**Implementation:**
```yaml
# docker-compose.yml (conceptual)
services:
  crucible:
    image: aeternum/crucible:latest
    volumes:
      - ./images:/images
      - ./results:/results
    environment:
      - PARALLELISM=16  # Run 16 VMs concurrently
    command: run-batch

# Crucible internals:
1. Spin up QEMU with machine config
2. Attach serial console logging
3. Set timeout (e.g., 60 seconds to reach shell)
4. Parse boot log for:
   - Kernel panic (failure)
   - Driver init errors (partial failure)
   - Shell prompt reached (success)
5. Write structured result to results/
```

**Success Criteria (phased):**
- **Phase 1:** Kernel boots, prints to serial
- **Phase 2:** Drivers initialize without panic
- **Phase 3:** Nebula shell renders
- **Phase 4:** User input works (keyboard/mouse)

---

### 2.6 The Oracle (Feedback Loop)

**Purpose:** Turn boot results into training signal.

**Inputs:**
- Boot results from Crucible
- Transpilation audit trails from Alchemist

**Outputs:**
- Updated skill confidence scores
- Failure patterns to avoid
- Retraining data for LLM component

**Logic:**
```
if boot_success:
    for skill in skills_used:
        skill.confidence += 1
        skill.validated_machines.append(machine_id)
    
elif boot_failure:
    analyze_panic_log()
    identify_failing_driver()
    
    if pattern_matched_incorrectly:
        skill.confidence -= 1
        skill.failure_cases.append(context)
    
    if novel_failure:
        queue_for_human_review()
        # Or: spawn agent to investigate
```

---

## 3. Implementation Roadmap

### Week 1-2: The Cartographer
- [ ] Clone Linux kernel (6.6 LTS)
- [ ] Write parser for MODULE_DEVICE_TABLE
- [ ] Generate initial `driver_manifest.json`
- [ ] Validate against pci.ids/usb.ids

### Week 3-4: The Architect + Crucible MVP
- [ ] Define hardware slot schema
- [ ] Generate 100 synthetic machines (simple combos)
- [ ] Write QEMU config generator
- [ ] Docker container that boots a Linux kernel
- [ ] Serial log capture + success detection

### Week 5-6: The Alchemist (Pattern Matcher)
- [ ] Identify 10 most common driver idioms
- [ ] Write rule-based transpiler for those patterns
- [ ] Test on simple drivers (USB HID mouse)
- [ ] Measure safe vs unsafe ratio

### Week 7-8: Integration Loop
- [ ] Cartographer → Architect → Alchemist → Foundry → Crucible pipeline
- [ ] Run overnight batch: 100 synthetic machines
- [ ] Analyze results, identify failure patterns
- [ ] First Oracle feedback cycle

### Month 3+: Scale & Learn
- [ ] Expand to 1000+ synthetic machines
- [ ] Add LLM fallback for complex drivers
- [ ] Implement skill accumulation
- [ ] Begin seL4 integration (replace Linux base)

---

## 4. Infrastructure Requirements

**The Forge (Rob's machine):**
- Primary development
- LLM inference for Alchemist
- Small-batch Crucible runs

**Cloud burst (optional):**
- Large-batch overnight runs (100s of VMs)
- Could use spot instances (cheap)
- Or: just run locally with patience

**Storage:**
- Linux kernel source: ~4GB
- Driver manifest + synthetic machines: ~100MB
- Compiled images: ~400MB each × N machines
- Plan for 1TB working space

---

## 5. Open Questions

1. **Which LLM for Alchemist?** Aurora (once ready)? Claude API? Local model?
2. **Kernel base for Phase 1?** Minimal Linux vs attempting seL4 early?
3. **How to handle GPU drivers?** (These are massive, proprietary-ish)
4. **Licensing?** Linux drivers are GPL—does transpiled Rust inherit that?
5. **When to involve real hardware?** After N successful VM boots?

---

## 6. Success Metrics

| Metric | Target (Phase 1) | Target (Phase 3) |
|--------|------------------|------------------|
| Drivers in manifest | 500+ | 5000+ |
| Synthetic machines generated | 100 | 10,000 |
| Boot success rate | 30% | 80% |
| Safe Rust ratio (avg) | 40% | 70% |
| Time to generate image | 10 min | 2 min |

---

## Next Immediate Action

**Today:** Set up the Cartographer.
```bash
git clone --depth 1 --branch v6.6 \
  https://git.kernel.org/pub/scm/linux/kernel/git/stable/linux.git

# Then: write the parser
```

---

*The Forge builds the builders.*
