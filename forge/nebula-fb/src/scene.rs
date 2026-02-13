/// Scene system — Scene trait and SceneManager with stack-based transitions.

use crate::input::InputEvent;
use crate::renderer::Renderer;
use crate::text::TextRenderer;

/// Transition instruction returned by scenes.
pub enum Transition {
    None,
    Push(Box<dyn Scene>),
    Replace(Box<dyn Scene>),
    Pop,
}

/// A single scene (screen) in the application.
pub trait Scene {
    /// Update logic. dt is seconds since last frame.
    fn update(&mut self, dt: f32) -> Transition;
    /// Draw the scene to the renderer.
    fn draw(&self, renderer: &mut Renderer, text: &TextRenderer);
    /// Handle an input event.
    fn handle_input(&mut self, event: InputEvent) -> Transition;
}

/// Manages a stack of scenes.
pub struct SceneManager {
    stack: Vec<Box<dyn Scene>>,
}

impl SceneManager {
    pub fn new(initial: Box<dyn Scene>) -> Self {
        Self {
            stack: vec![initial],
        }
    }

    pub fn update(&mut self, dt: f32) {
        let transition = if let Some(scene) = self.stack.last_mut() {
            scene.update(dt)
        } else {
            return;
        };
        self.apply(transition);
    }

    pub fn draw(&self, renderer: &mut Renderer, text: &TextRenderer) {
        if let Some(scene) = self.stack.last() {
            scene.draw(renderer, text);
        }
    }

    pub fn handle_input(&mut self, event: InputEvent) {
        let transition = if let Some(scene) = self.stack.last_mut() {
            scene.handle_input(event)
        } else {
            return;
        };
        self.apply(transition);
    }

    pub fn is_empty(&self) -> bool {
        self.stack.is_empty()
    }

    fn apply(&mut self, transition: Transition) {
        match transition {
            Transition::None => {}
            Transition::Push(scene) => self.stack.push(scene),
            Transition::Replace(scene) => {
                self.stack.pop();
                self.stack.push(scene);
            }
            Transition::Pop => {
                self.stack.pop();
            }
        }
    }
}
