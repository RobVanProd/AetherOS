//! Facets
//!
//! Facets are capabilities, not applications. They compose to accomplish tasks.
//! Each facet is a sandboxed WASM module with a defined interface.

use glam::Vec2;
use std::collections::HashMap;

use crate::render::Renderer;

/// Capabilities a facet can declare
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Capability {
    // Content
    ReadText,
    WriteText,
    ReadImage,
    WriteImage,
    ReadAudio,
    WriteAudio,
    ReadVideo,
    WriteVideo,
    
    // System
    FileAccess,
    NetworkAccess,
    Clipboard,
    Notifications,
    
    // Hardware
    Camera,
    Microphone,
    Location,
    
    // Integration
    LLMAccess,  // Talk to Aurora/Claude
    SemanticSearch,  // Query the vector store
}

/// Facet state
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FacetState {
    Loading,
    Active,
    Background,
    Suspended,
}

/// Data that can flow between facets
#[derive(Clone, Debug)]
pub enum FacetData {
    Text(String),
    Binary(Vec<u8>),
    Json(serde_json::Value),
    Reference { uri: String },
}

/// Facet trait - what every facet must implement
pub trait Facet: Send + Sync {
    /// Unique identifier
    fn id(&self) -> &str;
    
    /// Human-readable name
    fn name(&self) -> &str;
    
    /// What this facet can do
    fn capabilities(&self) -> Vec<Capability>;
    
    /// MIME types this facet accepts
    fn accepts(&self) -> Vec<&str>;
    
    /// MIME types this facet produces
    fn produces(&self) -> Vec<&str>;
    
    /// Initialize with optional data
    fn init(&mut self, data: Option<FacetData>);
    
    /// Update (called each frame)
    fn update(&mut self, dt: f32);
    
    /// Render to the given region
    fn render(&self, renderer: &mut Renderer, position: Vec2, size: Vec2);
    
    /// Handle text input
    fn on_text(&mut self, text: &str);
    
    /// Handle key press
    fn on_key(&mut self, key: crate::input::Key, pressed: bool);
    
    /// Receive data from another facet
    fn receive(&mut self, data: FacetData);
    
    /// Provide data to another facet
    fn provide(&self) -> Option<FacetData>;
    
    /// Suggest next action (for Omni-bar integration)
    fn suggest(&self) -> Option<String>;
}

/// Facet instance wrapper
pub struct FacetInstance {
    pub facet: Box<dyn Facet>,
    pub state: FacetState,
    pub position: Vec2,
    pub size: Vec2,
    pub z_index: i32,
}

/// Facet registry - knows about all available facets
pub struct FacetRegistry {
    factories: HashMap<String, Box<dyn Fn() -> Box<dyn Facet> + Send + Sync>>,
}

impl FacetRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            factories: HashMap::new(),
        };
        
        // Register built-in facets
        registry.register("terminal", || Box::new(TerminalFacet::new()));
        registry.register("editor", || Box::new(EditorFacet::new()));
        registry.register("files", || Box::new(FilesFacet::new()));
        
        registry
    }
    
    pub fn register<F>(&mut self, id: &str, factory: F)
    where
        F: Fn() -> Box<dyn Facet> + Send + Sync + 'static,
    {
        self.factories.insert(id.to_string(), Box::new(factory));
    }
    
    pub fn create(&self, id: &str) -> Option<Box<dyn Facet>> {
        self.factories.get(id).map(|f| f())
    }
    
    pub fn list(&self) -> Vec<&str> {
        self.factories.keys().map(|s| s.as_str()).collect()
    }
}

impl Default for FacetRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================
// Built-in Facets
// ============================================

/// Terminal facet - command line interface
pub struct TerminalFacet {
    history: Vec<String>,
    current_line: String,
    output: Vec<String>,
}

impl TerminalFacet {
    pub fn new() -> Self {
        Self {
            history: Vec::new(),
            current_line: String::new(),
            output: vec![
                "Aether Terminal v0.1".to_string(),
                "Type 'help' for commands".to_string(),
                "".to_string(),
            ],
        }
    }
}

impl Facet for TerminalFacet {
    fn id(&self) -> &str { "terminal" }
    fn name(&self) -> &str { "Terminal" }
    
    fn capabilities(&self) -> Vec<Capability> {
        vec![Capability::ReadText, Capability::WriteText, Capability::FileAccess]
    }
    
    fn accepts(&self) -> Vec<&str> { vec!["text/plain"] }
    fn produces(&self) -> Vec<&str> { vec!["text/plain"] }
    
    fn init(&mut self, data: Option<FacetData>) {
        if let Some(FacetData::Text(cmd)) = data {
            self.current_line = cmd;
        }
    }
    
    fn update(&mut self, _dt: f32) {}
    
    fn render(&self, renderer: &mut Renderer, position: Vec2, size: Vec2) {
        use crate::render::{Color, Rect};
        
        // Background
        renderer.draw_rect(
            Rect::new(position.x, position.y, size.x, size.y),
            Color::rgb(0.05, 0.05, 0.08),
            8.0,
        );
        
        // Output lines
        let line_height = 18.0;
        let padding = 12.0;
        let max_lines = ((size.y - padding * 2.0) / line_height) as usize;
        
        let start = self.output.len().saturating_sub(max_lines);
        for (i, line) in self.output[start..].iter().enumerate() {
            renderer.draw_text(
                line,
                Vec2::new(position.x + padding, position.y + padding + (i as f32 * line_height)),
                14.0,
                Color::rgb(0.8, 0.9, 0.8),
            );
        }
        
        // Current input line
        let prompt = format!("$ {}_", self.current_line);
        let y = position.y + size.y - padding - line_height;
        renderer.draw_text(
            &prompt,
            Vec2::new(position.x + padding, y),
            14.0,
            Color::rgb(0.6, 0.9, 0.6),
        );
    }
    
    fn on_text(&mut self, text: &str) {
        self.current_line.push_str(text);
    }
    
    fn on_key(&mut self, key: crate::input::Key, pressed: bool) {
        if !pressed { return; }
        
        match key {
            crate::input::Key::Enter => {
                let cmd = std::mem::take(&mut self.current_line);
                self.output.push(format!("$ {}", cmd));
                self.history.push(cmd.clone());
                
                // Execute command (simplified)
                let response = match cmd.as_str() {
                    "help" => "Commands: help, clear, echo, exit".to_string(),
                    "clear" => { self.output.clear(); String::new() }
                    s if s.starts_with("echo ") => s[5..].to_string(),
                    "" => String::new(),
                    _ => format!("Unknown command: {}", cmd),
                };
                
                if !response.is_empty() {
                    self.output.push(response);
                }
            }
            crate::input::Key::Backspace => {
                self.current_line.pop();
            }
            _ => {}
        }
    }
    
    fn receive(&mut self, data: FacetData) {
        if let FacetData::Text(cmd) = data {
            self.current_line = cmd;
        }
    }
    
    fn provide(&self) -> Option<FacetData> {
        Some(FacetData::Text(self.output.join("\n")))
    }
    
    fn suggest(&self) -> Option<String> {
        None
    }
}

/// Editor facet - text editing
pub struct EditorFacet {
    content: String,
    cursor: usize,
    filename: Option<String>,
}

impl EditorFacet {
    pub fn new() -> Self {
        Self {
            content: String::new(),
            cursor: 0,
            filename: None,
        }
    }
}

impl Facet for EditorFacet {
    fn id(&self) -> &str { "editor" }
    fn name(&self) -> &str { "Editor" }
    
    fn capabilities(&self) -> Vec<Capability> {
        vec![Capability::ReadText, Capability::WriteText, Capability::FileAccess]
    }
    
    fn accepts(&self) -> Vec<&str> { vec!["text/plain", "text/markdown"] }
    fn produces(&self) -> Vec<&str> { vec!["text/plain", "text/markdown"] }
    
    fn init(&mut self, data: Option<FacetData>) {
        if let Some(FacetData::Text(text)) = data {
            self.content = text;
            self.cursor = self.content.len();
        }
    }
    
    fn update(&mut self, _dt: f32) {}
    
    fn render(&self, renderer: &mut Renderer, position: Vec2, size: Vec2) {
        use crate::render::{Color, Rect};
        
        // Background
        renderer.draw_rect(
            Rect::new(position.x, position.y, size.x, size.y),
            Color::SURFACE,
            8.0,
        );
        
        // Title bar
        let title = self.filename.as_deref().unwrap_or("Untitled");
        renderer.draw_text(
            title,
            Vec2::new(position.x + 12.0, position.y + 8.0),
            12.0,
            Color::TEXT_DIM,
        );
        
        // Content
        let content_y = position.y + 32.0;
        let line_height = 20.0;
        
        for (i, line) in self.content.lines().enumerate() {
            renderer.draw_text(
                line,
                Vec2::new(position.x + 12.0, content_y + (i as f32 * line_height)),
                14.0,
                Color::TEXT,
            );
        }
    }
    
    fn on_text(&mut self, text: &str) {
        self.content.insert_str(self.cursor, text);
        self.cursor += text.len();
    }
    
    fn on_key(&mut self, key: crate::input::Key, pressed: bool) {
        if !pressed { return; }
        
        match key {
            crate::input::Key::Backspace => {
                if self.cursor > 0 {
                    self.cursor -= 1;
                    self.content.remove(self.cursor);
                }
            }
            crate::input::Key::Enter => {
                self.content.insert(self.cursor, '\n');
                self.cursor += 1;
            }
            crate::input::Key::Left => {
                self.cursor = self.cursor.saturating_sub(1);
            }
            crate::input::Key::Right => {
                self.cursor = (self.cursor + 1).min(self.content.len());
            }
            _ => {}
        }
    }
    
    fn receive(&mut self, data: FacetData) {
        if let FacetData::Text(text) = data {
            self.content = text;
            self.cursor = self.content.len();
        }
    }
    
    fn provide(&self) -> Option<FacetData> {
        Some(FacetData::Text(self.content.clone()))
    }
    
    fn suggest(&self) -> Option<String> {
        Some("Save document".to_string())
    }
}

/// Files facet - file browser
pub struct FilesFacet {
    current_path: String,
    entries: Vec<String>,
    selected: usize,
}

impl FilesFacet {
    pub fn new() -> Self {
        Self {
            current_path: "/".to_string(),
            entries: vec![
                "..".to_string(),
                "home/".to_string(),
                "etc/".to_string(),
                "tmp/".to_string(),
            ],
            selected: 0,
        }
    }
}

impl Facet for FilesFacet {
    fn id(&self) -> &str { "files" }
    fn name(&self) -> &str { "Files" }
    
    fn capabilities(&self) -> Vec<Capability> {
        vec![Capability::FileAccess]
    }
    
    fn accepts(&self) -> Vec<&str> { vec![] }
    fn produces(&self) -> Vec<&str> { vec!["text/uri-list"] }
    
    fn init(&mut self, data: Option<FacetData>) {
        if let Some(FacetData::Text(path)) = data {
            self.current_path = path;
            // Would reload entries here
        }
    }
    
    fn update(&mut self, _dt: f32) {}
    
    fn render(&self, renderer: &mut Renderer, position: Vec2, size: Vec2) {
        use crate::render::{Color, Rect};
        
        // Background
        renderer.draw_rect(
            Rect::new(position.x, position.y, size.x, size.y),
            Color::SURFACE,
            8.0,
        );
        
        // Path bar
        renderer.draw_text(
            &self.current_path,
            Vec2::new(position.x + 12.0, position.y + 8.0),
            12.0,
            Color::TEXT_DIM,
        );
        
        // Entries
        let entry_height = 28.0;
        let content_y = position.y + 32.0;
        
        for (i, entry) in self.entries.iter().enumerate() {
            let y = content_y + (i as f32 * entry_height);
            
            // Selection highlight
            if i == self.selected {
                renderer.draw_rect(
                    Rect::new(position.x + 4.0, y, size.x - 8.0, entry_height - 2.0),
                    Color::ACCENT.with_alpha(0.2),
                    4.0,
                );
            }
            
            // Entry name
            let icon = if entry.ends_with('/') { "📁" } else { "📄" };
            renderer.draw_text(
                &format!("{} {}", icon, entry),
                Vec2::new(position.x + 12.0, y + 6.0),
                14.0,
                Color::TEXT,
            );
        }
    }
    
    fn on_text(&mut self, _text: &str) {}
    
    fn on_key(&mut self, key: crate::input::Key, pressed: bool) {
        if !pressed { return; }
        
        match key {
            crate::input::Key::Up => {
                self.selected = self.selected.saturating_sub(1);
            }
            crate::input::Key::Down => {
                self.selected = (self.selected + 1).min(self.entries.len().saturating_sub(1));
            }
            crate::input::Key::Enter => {
                // Would navigate into directory or open file
            }
            _ => {}
        }
    }
    
    fn receive(&mut self, _data: FacetData) {}
    
    fn provide(&self) -> Option<FacetData> {
        self.entries.get(self.selected).map(|e| {
            FacetData::Text(format!("{}{}", self.current_path, e))
        })
    }
    
    fn suggest(&self) -> Option<String> {
        None
    }
}

// Helper trait for Color
trait ColorExt {
    fn with_alpha(self, a: f32) -> Self;
}

impl ColorExt for crate::render::Color {
    fn with_alpha(self, a: f32) -> Self {
        Self { a, ..self }
    }
}
