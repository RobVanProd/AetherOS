/// Boot splash — "AetherOS" fades in, then transitions to setup or dashboard.

use crate::input::InputEvent;
use crate::renderer::Renderer;
use crate::scene::{Scene, Transition};
use crate::text::TextRenderer;
use crate::theme;

const SPLASH_DURATION: f32 = 2.5;
const FADE_IN_DURATION: f32 = 1.0;
const SETUP_FILE: &str = "/tmp/aether_setup.json";

pub struct BootSplash {
    elapsed: f32,
    screen_width: u32,
    screen_height: u32,
}

impl BootSplash {
    pub fn new(screen_width: u32, screen_height: u32) -> Self {
        Self {
            elapsed: 0.0,
            screen_width,
            screen_height,
        }
    }

    fn is_first_boot() -> bool {
        !std::path::Path::new(SETUP_FILE).exists()
    }
}

impl Scene for BootSplash {
    fn update(&mut self, dt: f32) -> Transition {
        self.elapsed += dt;
        if self.elapsed >= SPLASH_DURATION {
            if Self::is_first_boot() {
                Transition::Replace(Box::new(
                    super::setup::SetupWizard::new(self.screen_width, self.screen_height),
                ))
            } else {
                Transition::Replace(Box::new(
                    super::dashboard::Dashboard::new(self.screen_width, self.screen_height),
                ))
            }
        } else {
            Transition::None
        }
    }

    fn draw(&self, renderer: &mut Renderer, text: &TextRenderer) {
        renderer.clear(theme::BG);

        // Fade in alpha
        let alpha = (self.elapsed / FADE_IN_DURATION).clamp(0.0, 1.0);
        let title_color = theme::Color::rgba(
            theme::ACCENT_BLUE.r,
            theme::ACCENT_BLUE.g,
            theme::ACCENT_BLUE.b,
            (alpha * 255.0) as u8,
        );
        let cy = self.screen_height as f32 / 2.0;

        // "AetherOS" — large centered title
        text.draw_centered(
            renderer,
            "AetherOS",
            0.0,
            cy - 30.0,
            self.screen_width as f32,
            theme::FONT_SIZE_TITLE * 1.5,
            title_color,
        );

        // "Initializing..." below
        if self.elapsed > 0.5 {
            let sub_alpha = ((self.elapsed - 0.5) / FADE_IN_DURATION).clamp(0.0, 1.0);
            let c = theme::Color::rgba(
                theme::TEXT_MUTED.r,
                theme::TEXT_MUTED.g,
                theme::TEXT_MUTED.b,
                (sub_alpha * 255.0) as u8,
            );
            text.draw_centered(
                renderer,
                "Initializing...",
                0.0,
                cy + 30.0,
                self.screen_width as f32,
                theme::FONT_SIZE_BODY,
                c,
            );
        }
    }

    fn handle_input(&mut self, _event: InputEvent) -> Transition {
        // Skip splash on any key
        self.elapsed = SPLASH_DURATION;
        Transition::None
    }
}
