//! Nebula: The Aether OS Interface Shell
//!
//! A next-generation interface that replaces windows with intent-driven
//! context composition.

mod canvas;
mod color;
mod facet;
mod input;
mod omnibar;
mod render;

use anyhow::Result;
use glam::Vec2;
use std::sync::Arc;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;
use winit::event::{ElementState, Event, WindowEvent};
use winit::event_loop::EventLoop;
use winit::keyboard::{Key as WinitKey, NamedKey};
use winit::window::{Window, WindowBuilder};

use crate::canvas::Canvas;
use crate::input::InputHandler;
use crate::omnibar::OmniBar;
use crate::render::Renderer;

/// Nebula shell state
struct Nebula {
    renderer: Option<Renderer>,
    window: Option<Arc<Window>>,
    canvas: Canvas,
    omnibar: OmniBar,
    input: InputHandler,
    running: bool,
    last_frame: instant::Instant,
}

impl Nebula {
    fn new() -> Self {
        Self {
            renderer: None,
            window: None,
            canvas: Canvas::new(),
            omnibar: OmniBar::new(),
            input: InputHandler::new().unwrap(),
            running: true,
            last_frame: instant::Instant::now(),
        }
    }

    fn handle_nebula_event(&mut self, event: input::Event) {
        use input::Event;

        match event {
            Event::Key { key, pressed } => {
                if pressed {
                    match key {
                        input::Key::Escape => {
                            if self.omnibar.is_visible() {
                                self.omnibar.hide();
                            } else {
                                self.running = false;
                            }
                        }
                        input::Key::Space if self.input.modifiers().meta => {
                            self.omnibar.toggle();
                        }
                        _ => {
                            if self.omnibar.is_visible() {
                                self.omnibar.handle_key(key);
                            }
                        }
                    }
                }
            }
            Event::Text(c) => {
                if self.omnibar.is_visible() {
                    self.omnibar.handle_char(c);
                }
            }
            Event::Pointer { position, .. } => {
                self.canvas.handle_pointer(position);
            }
            Event::Scroll { delta } => {
                self.canvas.handle_scroll(delta);
            }
            Event::Quit => {
                self.running = false;
            }
        }
    }

    fn update(&mut self) {
        let now = instant::Instant::now();
        let dt = now.duration_since(self.last_frame).as_secs_f32();
        self.last_frame = now;

        let dt = dt.min(0.1); // Cap delta time to avoid physics explosions

        self.omnibar.update(dt);
        self.canvas.update(dt);
    }

    fn render(&mut self) {
        if let Some(renderer) = &mut self.renderer {
            renderer.begin_frame();

            // Render canvas (content layer)
            self.canvas.render(renderer);

            // Render omnibar (overlay layer)
            if self.omnibar.is_visible() {
                self.omnibar.render(renderer);
            }

            if let Err(e) = renderer.end_frame() {
                tracing::error!("Render error: {}", e);
            }
        }
    }
}

fn map_winit_key(key: &WinitKey) -> input::Key {
    match key {
        WinitKey::Named(named) => match named {
            NamedKey::Space => input::Key::Space,
            NamedKey::Enter => input::Key::Enter,
            NamedKey::Escape => input::Key::Escape,
            NamedKey::Tab => input::Key::Tab,
            NamedKey::Backspace => input::Key::Backspace,
            NamedKey::Delete => input::Key::Delete,
            NamedKey::ArrowUp => input::Key::Up,
            NamedKey::ArrowDown => input::Key::Down,
            NamedKey::ArrowLeft => input::Key::Left,
            NamedKey::ArrowRight => input::Key::Right,
            NamedKey::Home => input::Key::Home,
            NamedKey::End => input::Key::End,
            NamedKey::PageUp => input::Key::PageUp,
            NamedKey::PageDown => input::Key::PageDown,
            NamedKey::Shift => input::Key::Shift,
            NamedKey::Control => input::Key::Control,
            NamedKey::Alt => input::Key::Alt,
            NamedKey::Super => input::Key::Meta,
            NamedKey::F1 => input::Key::F1,
            NamedKey::F2 => input::Key::F2,
            NamedKey::F3 => input::Key::F3,
            NamedKey::F4 => input::Key::F4,
            NamedKey::F5 => input::Key::F5,
            NamedKey::F6 => input::Key::F6,
            NamedKey::F7 => input::Key::F7,
            NamedKey::F8 => input::Key::F8,
            NamedKey::F9 => input::Key::F9,
            NamedKey::F10 => input::Key::F10,
            NamedKey::F11 => input::Key::F11,
            NamedKey::F12 => input::Key::F12,
            _ => input::Key::Unknown,
        },
        WinitKey::Character(ch) => {
            match ch.as_str() {
                "a" | "A" => input::Key::A,
                "b" | "B" => input::Key::B,
                "c" | "C" => input::Key::C,
                "d" | "D" => input::Key::D,
                "e" | "E" => input::Key::E,
                "f" | "F" => input::Key::F,
                "g" | "G" => input::Key::G,
                "h" | "H" => input::Key::H,
                "i" | "I" => input::Key::I,
                "j" | "J" => input::Key::J,
                "k" | "K" => input::Key::K,
                "l" | "L" => input::Key::L,
                "m" | "M" => input::Key::M,
                "n" | "N" => input::Key::N,
                "o" | "O" => input::Key::O,
                "p" | "P" => input::Key::P,
                "q" | "Q" => input::Key::Q,
                "r" | "R" => input::Key::R,
                "s" | "S" => input::Key::S,
                "t" | "T" => input::Key::T,
                "u" | "U" => input::Key::U,
                "v" | "V" => input::Key::V,
                "w" | "W" => input::Key::W,
                "x" | "X" => input::Key::X,
                "y" | "Y" => input::Key::Y,
                "z" | "Z" => input::Key::Z,
                "0" => input::Key::Num0,
                "1" => input::Key::Num1,
                "2" => input::Key::Num2,
                "3" => input::Key::Num3,
                "4" => input::Key::Num4,
                "5" => input::Key::Num5,
                "6" => input::Key::Num6,
                "7" => input::Key::Num7,
                "8" => input::Key::Num8,
                "9" => input::Key::Num9,
                " " => input::Key::Space,
                _ => input::Key::Unknown,
            }
        }
        _ => input::Key::Unknown,
    }
}

fn main() -> Result<()> {
    // Initialize logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    // Banner
    println!(
        r#"
    +=======================================+
    |           N E B U L A                 |
    |       The Aether Interface            |
    |           v0.1.0                      |
    +=======================================+
    "#
    );

    let event_loop = EventLoop::new()?;
    let mut app = Nebula::new();

    event_loop.run(move |event, elwt| {
        match event {
            Event::Resumed => {
                if app.window.is_some() {
                    return;
                }

                let window = WindowBuilder::new()
                    .with_title("Nebula - The Aether Interface")
                    .with_inner_size(winit::dpi::LogicalSize::new(1280, 800))
                    .build(elwt)
                    .expect("Failed to create window");

                let window = Arc::new(window);

                match Renderer::new(window.clone()) {
                    Ok(renderer) => {
                        info!("Window and renderer initialized");
                        app.renderer = Some(renderer);
                        app.window = Some(window);
                    }
                    Err(e) => {
                        tracing::error!("Failed to create renderer: {}", e);
                        elwt.exit();
                    }
                }
            }

            Event::WindowEvent { event: window_event, .. } => {
                match window_event {
                    WindowEvent::CloseRequested => {
                        info!("Close requested");
                        app.running = false;
                        elwt.exit();
                    }

                    WindowEvent::Resized(new_size) => {
                        if let Some(renderer) = &mut app.renderer {
                            renderer.resize(new_size.width, new_size.height);
                        }
                    }

                    WindowEvent::KeyboardInput { event: key_event, .. } => {
                        let pressed = key_event.state == ElementState::Pressed;
                        let key = map_winit_key(&key_event.logical_key);

                        app.input.inject(input::Event::Key { key, pressed });

                        if pressed {
                            if let WinitKey::Character(ref ch) = key_event.logical_key {
                                for c in ch.chars() {
                                    if !c.is_control() {
                                        app.input.inject(input::Event::Text(c));
                                    }
                                }
                            }
                        }

                        for ev in app.input.poll() {
                            app.handle_nebula_event(ev);
                        }

                        if !app.running {
                            elwt.exit();
                        }
                    }

                    WindowEvent::CursorMoved { position, .. } => {
                        app.input.inject(input::Event::Pointer {
                            position: Vec2::new(position.x as f32, position.y as f32),
                            button: None,
                            pressed: false,
                        });
                        for ev in app.input.poll() {
                            app.handle_nebula_event(ev);
                        }
                    }

                    WindowEvent::MouseInput { state, button, .. } => {
                        let btn = match button {
                            winit::event::MouseButton::Left => Some(input::MouseButton::Left),
                            winit::event::MouseButton::Right => Some(input::MouseButton::Right),
                            winit::event::MouseButton::Middle => Some(input::MouseButton::Middle),
                            _ => None,
                        };
                        if let Some(btn) = btn {
                            app.input.inject(input::Event::Pointer {
                                position: app.input.pointer_position(),
                                button: Some(btn),
                                pressed: state == ElementState::Pressed,
                            });
                            for ev in app.input.poll() {
                                app.handle_nebula_event(ev);
                            }
                        }
                    }

                    WindowEvent::MouseWheel { delta, .. } => {
                        let d = match delta {
                            winit::event::MouseScrollDelta::LineDelta(x, y) => Vec2::new(x * 20.0, y * 20.0),
                            winit::event::MouseScrollDelta::PixelDelta(p) => Vec2::new(p.x as f32, p.y as f32),
                        };
                        app.input.inject(input::Event::Scroll { delta: d });
                        for ev in app.input.poll() {
                            app.handle_nebula_event(ev);
                        }
                    }

                    WindowEvent::RedrawRequested => {
                        app.update();
                        app.render();
                    }

                    _ => {}
                }
            }

            Event::AboutToWait => {
                // Request continuous redraw for animation
                if let Some(window) = &app.window {
                    window.request_redraw();
                }
            }

            _ => {}
        }
    })?;

    Ok(())
}
