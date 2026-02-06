# Nebula: The Aether Interface

## Design Philosophy

### The Three Principles

**1. Invisible by Default**
The best interface is no interface. When you're focused, the screen shows only your content. No chrome. No distractions. No "desktop" waiting underneath.

**2. Intent Over Action**
You don't "open an app." You express what you want to accomplish. The system composes the right capabilities to make it happen.

**3. Context is Continuous**
Your work doesn't live in "windows" that you open and close. Context flows. Switching tasks doesn't destroy state—it shifts focus.

---

## Core Interactions

### The Void
When nothing is active, the screen is empty—a calm gradient or pure black. This is intentional. The computer waits for you, not the other way around.

### The Omni-Bar (⌘ + Space, or gesture from any edge)
A single luminous input field appears, center-screen.

```
┌─────────────────────────────────────────────────────────────┐
│                                                             │
│                    ░░░░░░░░░░░░░░░░░░░░░                    │
│                                                             │
│                         ▋                                   │
│                                                             │
│                    ░░░░░░░░░░░░░░░░░░░░░                    │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

You type. It understands.

- `write a proposal for the Martinez deal` → Writing facet appears, pre-loaded with Martinez context
- `video call with Sarah` → Communication facet, Sarah's contact ready
- `that chart from yesterday` → Retrieved instantly, no file path needed
- `play something calm` → Audio facet, mood-appropriate selection
- `code` → Your current project loads, cursor where you left off

### The Canvas
Content doesn't live in windows. It exists on an infinite canvas you navigate.

- **Pinch to zoom out** → See all active contexts as islands
- **Swipe** → Move between contexts fluidly
- **Drag content** → It flows between contexts, relationships preserved

### Facets (Not Apps)
A Facet is a capability, not an application.

| Traditional | Aether |
|-------------|--------|
| Open Word, create document | "Write" facet appears when you need to write |
| Open Chrome, go to URL | Content appears, browser is invisible |
| Open Terminal, type commands | Command facet inline with whatever you're doing |

Facets compose. Writing + Research + Citation = they're all present, spatially arranged, data flowing between them.

### The Periphery
Information that doesn't need attention lives at the edges—literally.

- **Time** → Subtle gradient shift, not a clock widget
- **Notifications** → Gentle glow at screen edge, expandable
- **System state** → Color temperature, not status bars

---

## Visual Language

### Typography First
Text is the primary interface element. Beautiful, readable, hierarchical.

```
Font stack:
  - Display: Inter Display (or custom)
  - Body: Inter
  - Mono: JetBrains Mono
  
Hierarchy through:
  - Size (not bold everywhere)
  - Weight (subtle variations)
  - Opacity (de-emphasize, don't hide)
```

### Color System
Adaptive. Not "light mode" and "dark mode"—a continuous spectrum.

```
Base: Derives from time of day and ambient light
  - Dawn: Warm cream
  - Day: Clean white with blue hints  
  - Dusk: Warm gray
  - Night: Deep charcoal, not pure black

Accent: User-chosen, applied sparingly
  - Interactive elements
  - Focus indicators
  - Emphasis

Semantic:
  - Success: Not green (too traffic-light)
  - Warning: Not yellow (too aggressive)
  - Derived from accent, shifted in hue space
```

### Motion
Everything moves, but nothing distracts.

```
Principles:
  - Physics-based (spring, not linear)
  - Meaningful (motion conveys information)
  - Interruptible (never trap the user in animation)
  
Timing:
  - Micro (hover, press): 50-100ms
  - Transition (appear, move): 200-300ms
  - Emphasis (attention): 400-600ms
```

### Depth
Subtle. Not drop shadows everywhere.

```
Layers:
  0: Canvas (content)
  1: Active facet (slight lift)
  2: Omni-bar (prominent, glowing edge)
  3: System overlay (blur underneath)

Blur:
  - Background blur for overlays (not frosted glass everywhere)
  - Focus blur for depth of field effect
  - Subtle, not Vista
```

---

## Technical Architecture

### Rendering Stack

```
┌─────────────────────────────────────┐
│           Nebula Shell              │  ← Rust, application logic
├─────────────────────────────────────┤
│         Vello / wgpu                │  ← 2D rendering, GPU accelerated
├─────────────────────────────────────┤
│         DRM/KMS                     │  ← Direct GPU access
├─────────────────────────────────────┤
│         Aether Kernel               │  ← Our Linux/seL4 base
└─────────────────────────────────────┘
```

**Why not Wayland?**
Wayland assumes the window paradigm. We're rejecting windows. We go direct to GPU.

**Why wgpu?**
- Rust native
- Cross-platform (Vulkan, Metal, DX12, WebGPU)
- Modern GPU features
- Active development

**Why Vello?**
- 2D vector rendering on GPU
- Resolution independent
- Handles text beautifully
- From the same team as Druid/Xilem

### Compositor

Nebula IS the compositor. There's no separate layer.

```rust
// Simplified architecture
struct Nebula {
    gpu: wgpu::Device,
    canvas: Canvas,           // Infinite scrollable space
    omnibar: OmniBar,         // The command interface
    facets: Vec<Facet>,       // Active capabilities
    context: ContextGraph,    // Relationships between content
    input: InputHandler,      // Unified input (touch, mouse, keyboard, gaze)
}

impl Nebula {
    fn frame(&mut self) {
        self.input.process();
        self.context.update();
        self.facets.iter_mut().for_each(|f| f.update());
        self.canvas.render(&self.gpu);
        self.omnibar.render(&self.gpu);
    }
}
```

### Input Model

Unified. Device-agnostic.

```rust
enum Intent {
    Navigate { direction: Vec2, velocity: f32 },
    Select { point: Vec2 },
    Command { text: String },
    Zoom { factor: f32, center: Vec2 },
    Dismiss,
}

// Mouse scroll, trackpad pinch, touch swipe, gaze + dwell
// All map to the same Intent types
```

### Facet Protocol (WASM)

Facets are sandboxed WASM modules with a defined interface.

```rust
// facet.wit (WebAssembly Interface Types)
interface facet {
    // Identity
    func name() -> string;
    func capabilities() -> list<capability>;
    
    // Lifecycle
    func init(context: context-ref);
    func update(dt: f32);
    func render() -> render-commands;
    
    // Interaction
    func handle-intent(intent: intent) -> response;
    
    // Data
    func accepts(mime-type: string) -> bool;
    func receive(data: blob);
    func provide() -> blob;
}
```

---

## Implementation Phases

### Phase 1: The Canvas (Week 1-2)
- [ ] DRM/KMS initialization in Rust
- [ ] wgpu context on bare metal
- [ ] Basic shape rendering (rect, rounded rect)
- [ ] Text rendering (font loading, glyph rasterization)
- [ ] Input handling (evdev → events)
- [ ] 60fps render loop

**Deliverable:** Colored rectangle and text on screen, responds to keyboard.

### Phase 2: The Omni-Bar (Week 3-4)
- [ ] Text input field
- [ ] Animated appearance (fade + scale)
- [ ] Command parsing (simple string matching first)
- [ ] Result display (list with selection)
- [ ] Action execution (launch facet, navigate)

**Deliverable:** Press key → bar appears → type → results show → select → action.

### Phase 3: First Facets (Week 5-6)
- [ ] Facet container (positioned, sized region)
- [ ] Terminal facet (existing terminal emulator in WASM)
- [ ] Text facet (simple editor)
- [ ] Facet switching via Omni-bar

**Deliverable:** Can open terminal, type commands, output displays.

### Phase 4: Context & Canvas (Week 7-8)
- [ ] Multiple facets simultaneously
- [ ] Spatial arrangement (not overlapping windows)
- [ ] Canvas navigation (pan, zoom)
- [ ] Context preservation (state persists)

**Deliverable:** Multiple facets on canvas, navigate between them.

### Phase 5: Polish (Week 9-10)
- [ ] Animation system (spring physics)
- [ ] Color system (adaptive theming)
- [ ] Typography refinement
- [ ] Blur/depth effects
- [ ] Peripheral UI (subtle system status)

**Deliverable:** Looks and feels next-gen.

---

## The Omni-Bar: Intelligence Layer

This is where Aurora connects.

```
User types: "the proposal I was working on"

Traditional: Fuzzy file search, maybe finds "proposal_v3_final_FINAL.docx"

Aether:
  1. Parse intent (Aurora or local model)
  2. Query semantic memory: recent + "proposal" + user's active projects
  3. Return: The actual document, in context, with related materials
  4. Facet composition: Writing facet + the document + research notes
```

The Omni-bar isn't search. It's a natural language interface to your digital context.

---

## Hardware Targets

### Minimum (prototype)
- Intel/AMD integrated graphics
- 1920x1080 display
- 4GB RAM

### Recommended
- Discrete GPU (even low-end)
- High-DPI display
- 8GB+ RAM

### Future
- Multiple displays
- Touch input
- Eye tracking (remember the Oracle scheduler)

---

## What This Isn't

- **Not a tiling window manager** (i3, Sway) — still window-centric
- **Not a traditional DE** (GNOME, KDE) — too much chrome
- **Not a launcher** (Raycast, Alfred) — those sit on top of broken paradigms
- **Not a novelty** (wobbly windows, 3D desktops) — effects without purpose

This is a rethinking of how humans and computers share a screen.

---

## Inspiration

- **Bret Victor's "Seeing Spaces"** — environment responds to context
- **Dynamicland** — computation as physical space
- **Notion** — everything is blocks that compose
- **Things 3** — invisible interface, content is the UI
- **Apple's spatial computing** — but without the goggles

---

## First Milestone

Boot Aether → See black screen → Press key → Omni-bar fades in → Type "hello" → Text appears on canvas → Press Escape → Bar fades → Content remains.

That's it. That's the proof of concept. Everything else builds from there.
