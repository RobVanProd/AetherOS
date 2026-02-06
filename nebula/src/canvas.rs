//! The Canvas
//!
//! An infinite 2D space where content lives. Not windows—fluid regions
//! that can be navigated, zoomed, and spatially arranged.

use glam::Vec2;

use crate::render::{Color, Rect, Renderer};

/// A region on the canvas containing content
#[derive(Clone, Debug)]
pub struct Region {
    pub id: u64,
    pub position: Vec2,
    pub size: Vec2,
    pub content: RegionContent,
}

/// What a region contains
#[derive(Clone, Debug)]
pub enum RegionContent {
    Empty,
    Text { content: String },
    Facet { name: String },
}

/// Camera for viewing the canvas
#[derive(Clone, Debug)]
pub struct Camera {
    pub position: Vec2,
    pub zoom: f32,
    velocity: Vec2,
    zoom_velocity: f32,
}

impl Camera {
    fn new() -> Self {
        Self {
            position: Vec2::ZERO,
            zoom: 1.0,
            velocity: Vec2::ZERO,
            zoom_velocity: 0.0,
        }
    }

    fn update(&mut self, dt: f32) {
        // Apply velocity with damping
        let damping = 5.0;
        
        self.position += self.velocity * dt;
        self.velocity *= (-damping * dt).exp();
        
        self.zoom += self.zoom_velocity * dt;
        self.zoom_velocity *= (-damping * dt).exp();
        
        // Clamp zoom
        self.zoom = self.zoom.clamp(0.1, 5.0);
        
        // Stop when slow enough
        if self.velocity.length() < 0.1 {
            self.velocity = Vec2::ZERO;
        }
        if self.zoom_velocity.abs() < 0.001 {
            self.zoom_velocity = 0.0;
        }
    }

    fn pan(&mut self, delta: Vec2) {
        self.velocity += delta * 10.0;
    }

    fn zoom_by(&mut self, factor: f32) {
        self.zoom_velocity += factor;
    }

    /// Transform world coordinates to screen coordinates
    fn world_to_screen(&self, world: Vec2, screen_center: Vec2) -> Vec2 {
        (world - self.position) * self.zoom + screen_center
    }

    /// Transform screen coordinates to world coordinates  
    fn screen_to_world(&self, screen: Vec2, screen_center: Vec2) -> Vec2 {
        (screen - screen_center) / self.zoom + self.position
    }
}

/// The Canvas
pub struct Canvas {
    regions: Vec<Region>,
    camera: Camera,
    next_id: u64,
    pointer_pos: Vec2,
}

impl Canvas {
    pub fn new() -> Self {
        let mut canvas = Self {
            regions: Vec::new(),
            camera: Camera::new(),
            next_id: 1,
            pointer_pos: Vec2::ZERO,
        };
        
        // Add some initial content for testing
        canvas.add_region(
            Vec2::new(0.0, 0.0),
            Vec2::new(400.0, 200.0),
            RegionContent::Text {
                content: "Welcome to Aether.\n\nPress ⌘+Space to open the Omni-bar.\nType to search or command.\n\nThe future of computing is intent-driven.".to_string(),
            },
        );
        
        canvas
    }

    pub fn add_region(&mut self, position: Vec2, size: Vec2, content: RegionContent) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        
        self.regions.push(Region {
            id,
            position,
            size,
            content,
        });
        
        id
    }

    pub fn handle_pointer(&mut self, position: Vec2) {
        self.pointer_pos = position;
    }

    pub fn handle_scroll(&mut self, delta: Vec2) {
        // Vertical scroll = zoom, horizontal scroll = pan
        if delta.y.abs() > delta.x.abs() {
            self.camera.zoom_by(delta.y * 0.01);
        } else {
            self.camera.pan(Vec2::new(delta.x, 0.0));
        }
    }

    pub fn pan(&mut self, delta: Vec2) {
        self.camera.pan(delta);
    }

    pub fn update(&mut self, dt: f32) {
        self.camera.update(dt);
    }

    pub fn render(&self, renderer: &mut Renderer) {
        let screen_center = renderer.center();
        
        // Render each region
        for region in &self.regions {
            let screen_pos = self.camera.world_to_screen(region.position, screen_center);
            let screen_size = region.size * self.camera.zoom;
            
            // Culling: skip if off screen
            if screen_pos.x + screen_size.x < 0.0
                || screen_pos.x > renderer.width() as f32
                || screen_pos.y + screen_size.y < 0.0
                || screen_pos.y > renderer.height() as f32
            {
                continue;
            }
            
            let rect = Rect::new(screen_pos.x, screen_pos.y, screen_size.x, screen_size.y);
            
            // Region background
            renderer.draw_rect(
                rect,
                Color::rgba(Color::SURFACE.r, Color::SURFACE.g, Color::SURFACE.b, 0.8),
                8.0 * self.camera.zoom,
            );
            
            // Region content
            match &region.content {
                RegionContent::Empty => {}
                RegionContent::Text { content } => {
                    let padding = 16.0 * self.camera.zoom;
                    let font_size = 14.0 * self.camera.zoom;
                    
                    // Simple text rendering (would need proper line wrapping)
                    for (i, line) in content.lines().enumerate() {
                        renderer.draw_text(
                            line,
                            Vec2::new(
                                screen_pos.x + padding,
                                screen_pos.y + padding + (i as f32 * font_size * 1.5),
                            ),
                            font_size,
                            Color::TEXT,
                        );
                    }
                }
                RegionContent::Facet { name } => {
                    // Facets would render their own content
                    renderer.draw_text(
                        &format!("[Facet: {}]", name),
                        Vec2::new(screen_pos.x + 16.0, screen_pos.y + 16.0),
                        14.0 * self.camera.zoom,
                        Color::TEXT_DIM,
                    );
                }
            }
        }
        
        // Debug: show zoom level
        renderer.draw_text(
            &format!("Zoom: {:.1}x", self.camera.zoom),
            Vec2::new(16.0, renderer.height() as f32 - 32.0),
            12.0,
            Color::TEXT_DIM,
        );
    }
}

impl Default for Canvas {
    fn default() -> Self {
        Self::new()
    }
}
