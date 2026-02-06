//! Input Handler
//!
//! Unified input processing for keyboard, mouse, touch, and eventually gaze.
//! Converts raw device events into semantic intents.

use glam::Vec2;
use std::collections::HashSet;

/// Keyboard keys we care about
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Key {
    // Letters
    A, B, C, D, E, F, G, H, I, J, K, L, M,
    N, O, P, Q, R, S, T, U, V, W, X, Y, Z,
    
    // Numbers
    Num0, Num1, Num2, Num3, Num4, Num5, Num6, Num7, Num8, Num9,
    
    // Special
    Space,
    Enter,
    Escape,
    Tab,
    Backspace,
    Delete,
    
    // Navigation
    Up, Down, Left, Right,
    Home, End,
    PageUp, PageDown,
    
    // Modifiers
    Shift, Control, Alt, Meta,  // Meta = Super/Windows/Command
    
    // Function
    F1, F2, F3, F4, F5, F6, F7, F8, F9, F10, F11, F12,
    
    // Other
    Unknown,
}

/// Active modifier keys
#[derive(Clone, Copy, Debug, Default)]
pub struct Modifiers {
    pub shift: bool,
    pub control: bool,
    pub alt: bool,
    pub meta: bool,
}

impl Modifiers {
    pub fn any(&self) -> bool {
        self.shift || self.control || self.alt || self.meta
    }
}

/// Mouse buttons
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

/// Input events
#[derive(Clone, Debug)]
pub enum Event {
    Key {
        key: Key,
        pressed: bool,
    },
    Text(char),
    Pointer {
        position: Vec2,
        button: Option<MouseButton>,
        pressed: bool,
    },
    Scroll {
        delta: Vec2,
    },
    Quit,
}

/// Input handler
pub struct InputHandler {
    modifiers: Modifiers,
    pressed_keys: HashSet<Key>,
    pointer_position: Vec2,
    pressed_buttons: HashSet<MouseButton>,
    pending_events: Vec<Event>,
}

impl InputHandler {
    pub fn new() -> anyhow::Result<Self> {
        Ok(Self {
            modifiers: Modifiers::default(),
            pressed_keys: HashSet::new(),
            pointer_position: Vec2::ZERO,
            pressed_buttons: HashSet::new(),
            pending_events: Vec::new(),
        })
    }

    pub fn modifiers(&self) -> Modifiers {
        self.modifiers
    }

    pub fn is_key_pressed(&self, key: Key) -> bool {
        self.pressed_keys.contains(&key)
    }

    pub fn pointer_position(&self) -> Vec2 {
        self.pointer_position
    }

    pub fn is_button_pressed(&self, button: MouseButton) -> bool {
        self.pressed_buttons.contains(&button)
    }

    /// Poll for events (non-blocking)
    pub fn poll(&mut self) -> Vec<Event> {
        // In a real implementation, this would read from evdev or winit
        // For now, return pending events and simulate some basic input
        
        let events = std::mem::take(&mut self.pending_events);
        
        // Process events to update state
        for event in &events {
            match event {
                Event::Key { key, pressed } => {
                    if *pressed {
                        self.pressed_keys.insert(*key);
                    } else {
                        self.pressed_keys.remove(key);
                    }
                    
                    // Update modifiers
                    match key {
                        Key::Shift => self.modifiers.shift = *pressed,
                        Key::Control => self.modifiers.control = *pressed,
                        Key::Alt => self.modifiers.alt = *pressed,
                        Key::Meta => self.modifiers.meta = *pressed,
                        _ => {}
                    }
                }
                Event::Pointer { position, button, pressed } => {
                    self.pointer_position = *position;
                    if let Some(btn) = button {
                        if *pressed {
                            self.pressed_buttons.insert(*btn);
                        } else {
                            self.pressed_buttons.remove(btn);
                        }
                    }
                }
                _ => {}
            }
        }
        
        events
    }

    /// Inject an event (for testing or from external sources)
    pub fn inject(&mut self, event: Event) {
        self.pending_events.push(event);
    }
}

impl Default for InputHandler {
    fn default() -> Self {
        Self::new().unwrap()
    }
}

/// Convert evdev key codes to our Key enum
#[cfg(target_os = "linux")]
pub fn evdev_to_key(code: u16) -> Key {
    // Maps evdev key codes to our Key enum
    // Simplified version:
    match code {
        1 => Key::Escape,
        14 => Key::Backspace,
        15 => Key::Tab,
        28 => Key::Enter,
        29 => Key::Control,
        42 => Key::Shift,
        54 => Key::Shift,  // Right shift
        56 => Key::Alt,
        57 => Key::Space,
        // ... etc
        _ => Key::Unknown,
    }
}
