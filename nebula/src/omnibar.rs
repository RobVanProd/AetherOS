//! The Omni-Bar
//!
//! The single entry point for all user intent. Always one gesture away.
//! Understands natural language, commands, search, and navigation.

use glam::Vec2;

use crate::input::Key;
use crate::render::{Color, Rect, Renderer};

/// Animation state
#[derive(Clone, Copy, Debug)]
struct Animation {
    current: f32,
    target: f32,
    velocity: f32,
}

impl Animation {
    fn new(initial: f32) -> Self {
        Self {
            current: initial,
            target: initial,
            velocity: 0.0,
        }
    }

    fn set_target(&mut self, target: f32) {
        self.target = target;
    }

    fn update(&mut self, dt: f32) {
        // Spring physics
        let stiffness = 300.0;
        let damping = 20.0;

        let displacement = self.target - self.current;
        let spring_force = displacement * stiffness;
        let damping_force = self.velocity * damping;

        let acceleration = spring_force - damping_force;
        self.velocity += acceleration * dt;
        self.current += self.velocity * dt;

        // Snap when close enough
        if (self.target - self.current).abs() < 0.001 && self.velocity.abs() < 0.001 {
            self.current = self.target;
            self.velocity = 0.0;
        }
    }

    fn value(&self) -> f32 {
        self.current
    }
}

/// Search/command result
#[derive(Clone, Debug)]
pub struct OmniResult {
    pub title: String,
    pub subtitle: Option<String>,
    pub icon: Option<String>,
    pub action: OmniAction,
}

/// What happens when a result is selected
#[derive(Clone, Debug)]
pub enum OmniAction {
    OpenFacet { name: String },
    Execute { command: String },
    Navigate { path: String },
    Search { query: String },
}

/// The Omni-Bar
pub struct OmniBar {
    visible: bool,
    input_text: String,
    cursor_pos: usize,
    results: Vec<OmniResult>,
    selected_index: usize,

    // Animations
    opacity: Animation,
    scale: Animation,
    y_offset: Animation,
}

impl OmniBar {
    pub fn new() -> Self {
        Self {
            visible: false,
            input_text: String::new(),
            cursor_pos: 0,
            results: Vec::new(),
            selected_index: 0,
            opacity: Animation::new(0.0),
            scale: Animation::new(0.95),
            y_offset: Animation::new(-20.0),
        }
    }

    pub fn is_visible(&self) -> bool {
        self.visible || self.opacity.value() > 0.01
    }

    pub fn show(&mut self) {
        self.visible = true;
        self.input_text.clear();
        self.cursor_pos = 0;
        self.results.clear();
        self.selected_index = 0;

        // Animate in
        self.opacity.set_target(1.0);
        self.scale.set_target(1.0);
        self.y_offset.set_target(0.0);
    }

    pub fn hide(&mut self) {
        self.visible = false;

        // Animate out
        self.opacity.set_target(0.0);
        self.scale.set_target(0.95);
        self.y_offset.set_target(-20.0);
    }

    pub fn toggle(&mut self) {
        if self.visible {
            self.hide();
        } else {
            self.show();
        }
    }

    pub fn handle_key(&mut self, key: Key) {
        match key {
            Key::Backspace => {
                if self.cursor_pos > 0 {
                    self.cursor_pos -= 1;
                    self.input_text.remove(self.cursor_pos);
                    self.update_results();
                }
            }
            Key::Delete => {
                if self.cursor_pos < self.input_text.len() {
                    self.input_text.remove(self.cursor_pos);
                    self.update_results();
                }
            }
            Key::Left => {
                if self.cursor_pos > 0 {
                    self.cursor_pos -= 1;
                }
            }
            Key::Right => {
                if self.cursor_pos < self.input_text.len() {
                    self.cursor_pos += 1;
                }
            }
            Key::Up => {
                if self.selected_index > 0 {
                    self.selected_index -= 1;
                }
            }
            Key::Down => {
                if self.selected_index < self.results.len().saturating_sub(1) {
                    self.selected_index += 1;
                }
            }
            Key::Enter => {
                self.execute_selected();
            }
            Key::Tab => {
                // Autocomplete
                if let Some(result) = self.results.get(self.selected_index) {
                    self.input_text = result.title.clone();
                    self.cursor_pos = self.input_text.len();
                }
            }
            _ => {}
        }
    }

    pub fn handle_char(&mut self, c: char) {
        if c.is_control() {
            return;
        }

        self.input_text.insert(self.cursor_pos, c);
        self.cursor_pos += 1;
        self.update_results();
    }

    fn update_results(&mut self) {
        // Parse input and generate results
        // This is where Aurora/LLM integration would go

        self.results.clear();
        self.selected_index = 0;

        if self.input_text.is_empty() {
            return;
        }

        let query = self.input_text.to_lowercase();

        // Built-in commands
        if query.starts_with("term") || query.starts_with("shell") {
            self.results.push(OmniResult {
                title: "Terminal".to_string(),
                subtitle: Some("Open command line".to_string()),
                icon: Some("terminal".to_string()),
                action: OmniAction::OpenFacet {
                    name: "terminal".to_string(),
                },
            });
        }

        if query.starts_with("write") || query.starts_with("edit") || query.starts_with("note") {
            self.results.push(OmniResult {
                title: "Write".to_string(),
                subtitle: Some("Open text editor".to_string()),
                icon: Some("edit".to_string()),
                action: OmniAction::OpenFacet {
                    name: "editor".to_string(),
                },
            });
        }

        if query.starts_with("file") || query.starts_with("browse") {
            self.results.push(OmniResult {
                title: "Files".to_string(),
                subtitle: Some("Browse filesystem".to_string()),
                icon: Some("folder".to_string()),
                action: OmniAction::OpenFacet {
                    name: "files".to_string(),
                },
            });
        }

        if query.starts_with("set") || query.starts_with("pref") {
            self.results.push(OmniResult {
                title: "Settings".to_string(),
                subtitle: Some("System preferences".to_string()),
                icon: Some("settings".to_string()),
                action: OmniAction::OpenFacet {
                    name: "settings".to_string(),
                },
            });
        }

        // System commands
        if query == "quit" || query == "exit" || query == "logout" {
            self.results.push(OmniResult {
                title: "Quit Nebula".to_string(),
                subtitle: Some("Exit to console".to_string()),
                icon: Some("power".to_string()),
                action: OmniAction::Execute {
                    command: "quit".to_string(),
                },
            });
        }

        // Fallback: treat as search
        if self.results.is_empty() && self.input_text.len() > 2 {
            self.results.push(OmniResult {
                title: format!("Search for \"{}\"", self.input_text),
                subtitle: Some("Search files and content".to_string()),
                icon: Some("search".to_string()),
                action: OmniAction::Search {
                    query: self.input_text.clone(),
                },
            });
        }
    }

    fn execute_selected(&mut self) {
        if let Some(result) = self.results.get(self.selected_index) {
            match &result.action {
                OmniAction::OpenFacet { name } => {
                    tracing::info!("Opening facet: {}", name);
                    // TODO: Actually open facet
                }
                OmniAction::Execute { command } => {
                    tracing::info!("Executing: {}", command);
                    // TODO: Execute command
                }
                OmniAction::Navigate { path } => {
                    tracing::info!("Navigating to: {}", path);
                    // TODO: Navigate
                }
                OmniAction::Search { query } => {
                    tracing::info!("Searching: {}", query);
                    // TODO: Search
                }
            }
        }

        self.hide();
    }

    pub fn update(&mut self, dt: f32) {
        self.opacity.update(dt);
        self.scale.update(dt);
        self.y_offset.update(dt);
    }

    pub fn render(&self, renderer: &mut Renderer) {
        let opacity = self.opacity.value();
        if opacity < 0.01 {
            return;
        }

        let scale = self.scale.value();
        let y_offset = self.y_offset.value();

        let center = renderer.center();
        let bar_width = 600.0 * scale;
        let bar_height = 56.0 * scale;

        // Background blur (conceptual - actual blur requires shader)
        let blur_rect = Rect::centered(
            Vec2::new(center.x, center.y * 0.4 + y_offset),
            bar_width + 40.0,
            bar_height + 40.0 + (self.results.len() as f32 * 48.0),
        );
        renderer.draw_blur(blur_rect, 20.0);

        // Bar background
        let bar_rect = Rect::centered(
            Vec2::new(center.x, center.y * 0.4 + y_offset),
            bar_width,
            bar_height,
        );

        // Glow effect (draw slightly larger rect behind)
        let glow_rect = Rect::centered(
            Vec2::new(center.x, center.y * 0.4 + y_offset),
            bar_width + 4.0,
            bar_height + 4.0,
        );
        renderer.draw_rect(
            glow_rect,
            Color::rgba(
                Color::ACCENT.r,
                Color::ACCENT.g,
                Color::ACCENT.b,
                0.3 * opacity,
            ),
            16.0,
        );

        // Main bar
        renderer.draw_rect(
            bar_rect,
            Color::rgba(
                Color::SURFACE.r,
                Color::SURFACE.g,
                Color::SURFACE.b,
                0.95 * opacity,
            ),
            12.0,
        );

        // Input text
        let text = if self.input_text.is_empty() {
            "What would you like to do?"
        } else {
            &self.input_text
        };

        let text_color = if self.input_text.is_empty() {
            Color::rgba(
                Color::TEXT_DIM.r,
                Color::TEXT_DIM.g,
                Color::TEXT_DIM.b,
                opacity,
            )
        } else {
            Color::rgba(Color::TEXT.r, Color::TEXT.g, Color::TEXT.b, opacity)
        };

        renderer.draw_text(
            text,
            Vec2::new(bar_rect.x + 20.0, bar_rect.y + bar_height / 2.0 - 10.0),
            20.0 * scale,
            text_color,
        );

        // Cursor
        if self.visible && !self.input_text.is_empty() {
            // Simple cursor rendering (would need proper text measurement)
            let cursor_x = bar_rect.x + 20.0 + (self.cursor_pos as f32 * 10.0);
            renderer.draw_rect(
                Rect::new(cursor_x, bar_rect.y + 14.0, 2.0, bar_height - 28.0),
                Color::rgba(Color::ACCENT.r, Color::ACCENT.g, Color::ACCENT.b, opacity),
                1.0,
            );
        }

        // Results
        if !self.results.is_empty() {
            let results_y = bar_rect.y + bar_height + 8.0;

            for (i, result) in self.results.iter().enumerate() {
                let result_rect = Rect::new(
                    bar_rect.x,
                    results_y + (i as f32 * 48.0),
                    bar_width,
                    44.0,
                );

                // Selected highlight
                if i == self.selected_index {
                    renderer.draw_rect(
                        result_rect,
                        Color::rgba(
                            Color::ACCENT.r,
                            Color::ACCENT.g,
                            Color::ACCENT.b,
                            0.2 * opacity,
                        ),
                        8.0,
                    );
                }

                // Title
                renderer.draw_text(
                    &result.title,
                    Vec2::new(result_rect.x + 16.0, result_rect.y + 12.0),
                    16.0 * scale,
                    Color::rgba(Color::TEXT.r, Color::TEXT.g, Color::TEXT.b, opacity),
                );

                // Subtitle
                if let Some(subtitle) = &result.subtitle {
                    renderer.draw_text(
                        subtitle,
                        Vec2::new(result_rect.x + 16.0, result_rect.y + 28.0),
                        12.0 * scale,
                        Color::rgba(
                            Color::TEXT_DIM.r,
                            Color::TEXT_DIM.g,
                            Color::TEXT_DIM.b,
                            opacity,
                        ),
                    );
                }
            }
        }
    }
}

impl Default for OmniBar {
    fn default() -> Self {
        Self::new()
    }
}
