/// Setup wizard — 3-step first-boot experience.
/// Step 1: Name input
/// Step 2: Interest chips selection
/// Step 3: Animated progress "Setting up your experience..."

use crate::input::InputEvent;
use crate::renderer::Renderer;
use crate::scene::{Scene, Transition};
use crate::text::TextRenderer;
use crate::theme;
use crate::widgets::button;
use crate::widgets::progress;

const SETUP_FILE: &str = "/tmp/aether_setup.json";

const INTEREST_OPTIONS: &[&str] = &[
    "Technology",
    "Weather",
    "Science",
    "Programming",
    "News",
    "Finance",
    "Music",
    "Space",
    "AI",
    "Health",
];

#[derive(Clone, Copy, PartialEq)]
enum Step {
    Name,
    Interests,
    Finishing,
}

pub struct SetupWizard {
    screen_width: u32,
    screen_height: u32,
    step: Step,
    name: String,
    cursor: usize,
    selected_interests: Vec<bool>,
    interest_cursor: usize,
    finish_elapsed: f32,
    finish_duration: f32,
}

impl SetupWizard {
    pub fn new(screen_width: u32, screen_height: u32) -> Self {
        Self {
            screen_width,
            screen_height,
            step: Step::Name,
            name: String::new(),
            cursor: 0,
            selected_interests: vec![false; INTEREST_OPTIONS.len()],
            interest_cursor: 0,
            finish_elapsed: 0.0,
            finish_duration: 3.0,
        }
    }

    fn save_setup(&self) {
        let interests: Vec<&str> = INTEREST_OPTIONS
            .iter()
            .zip(self.selected_interests.iter())
            .filter(|(_, &sel)| sel)
            .map(|(&name, _)| name)
            .collect();

        let data = serde_json::json!({
            "name": self.name,
            "interests": interests,
        });

        if let Ok(json) = serde_json::to_string_pretty(&data) {
            let _ = std::fs::write(SETUP_FILE, json);
        }
    }
}

impl Scene for SetupWizard {
    fn update(&mut self, dt: f32) -> Transition {
        if self.step == Step::Finishing {
            self.finish_elapsed += dt;
            if self.finish_elapsed >= self.finish_duration {
                self.save_setup();
                return Transition::Replace(Box::new(
                    super::dashboard::Dashboard::new(self.screen_width, self.screen_height),
                ));
            }
        }
        Transition::None
    }

    fn draw(&self, renderer: &mut Renderer, text: &TextRenderer) {
        renderer.clear(theme::BG);

        let cx = self.screen_width as f32 / 2.0;
        let w = self.screen_width as f32;

        match self.step {
            Step::Name => {
                // Title
                text.draw_centered(renderer, "Welcome to AetherOS", 0.0, 200.0, w, theme::FONT_SIZE_TITLE, theme::TEXT_PRIMARY);
                text.draw_centered(renderer, "What should we call you?", 0.0, 250.0, w, theme::FONT_SIZE_BODY, theme::TEXT_SECONDARY);

                // Name input box
                let box_w = 400.0;
                let box_h = 48.0;
                let box_x = cx - box_w / 2.0;
                let box_y = 320.0;
                renderer.fill_rounded_rect(box_x, box_y, box_w, box_h, 8.0, theme::SURFACE);
                renderer.stroke_rounded_rect(box_x, box_y, box_w, box_h, 8.0, theme::ACCENT_BLUE, 2.0);

                if self.name.is_empty() {
                    text.draw(renderer, "Your name", box_x + 16.0, box_y + 14.0, theme::FONT_SIZE_BODY, theme::TEXT_MUTED);
                } else {
                    text.draw(renderer, &self.name, box_x + 16.0, box_y + 14.0, theme::FONT_SIZE_BODY, theme::TEXT_PRIMARY);
                }
                // Cursor
                let cursor_x = box_x + 16.0 + text.measure(&self.name[..self.cursor], theme::FONT_SIZE_BODY);
                renderer.fill_rect(cursor_x, box_y + 12.0, 2.0, 24.0, theme::ACCENT_BLUE);

                // Continue button
                let btn_label = "Continue";
                let btn_w = text.measure(btn_label, theme::FONT_SIZE_BODY) + 24.0;
                button::draw_button(renderer, text, btn_label, cx - btn_w / 2.0, 400.0, !self.name.is_empty());
            }

            Step::Interests => {
                text.draw_centered(renderer, "What are you interested in?", 0.0, 200.0, w, theme::FONT_SIZE_TITLE, theme::TEXT_PRIMARY);
                text.draw_centered(renderer, "Select topics to personalize your experience.", 0.0, 250.0, w, theme::FONT_SIZE_BODY, theme::TEXT_SECONDARY);

                // Chip grid
                let grid_w = 700.0;
                let start_x = cx - grid_w / 2.0;
                let mut chip_x = start_x;
                let mut chip_y = 320.0;
                let gap = 12.0;

                for (i, &label) in INTEREST_OPTIONS.iter().enumerate() {
                    let is_selected = self.selected_interests[i];
                    let is_cursor = i == self.interest_cursor;

                    let (cw, ch) = button::draw_chip(renderer, text, label, chip_x, chip_y, is_selected);

                    // Cursor indicator
                    if is_cursor {
                        renderer.stroke_rounded_rect(chip_x - 2.0, chip_y - 2.0, cw + 4.0, ch + 4.0, (ch + 4.0) / 2.0, theme::ACCENT_BLUE, 1.5);
                    }

                    chip_x += cw + gap;
                    if chip_x + 100.0 > start_x + grid_w {
                        chip_x = start_x;
                        chip_y += ch + gap;
                    }
                }

                // Continue button
                let any_selected = self.selected_interests.iter().any(|&s| s);
                let btn_y = chip_y + 60.0;
                let btn_label = "Continue";
                let btn_w = text.measure(btn_label, theme::FONT_SIZE_BODY) + 24.0;
                button::draw_button(renderer, text, btn_label, cx - btn_w / 2.0, btn_y, any_selected);

                text.draw_centered(
                    renderer,
                    "Navigate: Arrow keys  |  Select: Enter/Space  |  Continue: Tab",
                    0.0,
                    btn_y + 60.0,
                    w,
                    theme::FONT_SIZE_SMALL,
                    theme::TEXT_MUTED,
                );
            }

            Step::Finishing => {
                let progress_val = (self.finish_elapsed / self.finish_duration).clamp(0.0, 1.0);

                text.draw_centered(renderer, "Setting up your experience...", 0.0, 300.0, w, theme::FONT_SIZE_HEADING, theme::TEXT_PRIMARY);

                let bar_w = 500.0;
                progress::draw_progress_animated(
                    renderer,
                    cx - bar_w / 2.0,
                    370.0,
                    bar_w,
                    16.0,
                    progress_val,
                    self.finish_elapsed,
                );

                let pct_text = format!("{:.0}%", progress_val * 100.0);
                text.draw_centered(renderer, &pct_text, 0.0, 400.0, w, theme::FONT_SIZE_SMALL, theme::TEXT_SECONDARY);

                // Show what's being "set up"
                let steps = ["Loading preferences...", "Connecting to AI...", "Building your dashboard..."];
                let step_idx = ((progress_val * steps.len() as f32) as usize).min(steps.len() - 1);
                text.draw_centered(renderer, steps[step_idx], 0.0, 430.0, w, theme::FONT_SIZE_SMALL, theme::TEXT_MUTED);
            }
        }

        // Step indicator dots at bottom
        let dot_y = self.screen_height as f32 - 60.0;
        let steps = [Step::Name, Step::Interests, Step::Finishing];
        let total_w = steps.len() as f32 * 12.0 + (steps.len() - 1) as f32 * 8.0;
        let mut dx = cx - total_w / 2.0;
        for &s in &steps {
            let color = if s == self.step { theme::ACCENT_BLUE } else { theme::TEXT_MUTED };
            renderer.fill_rounded_rect(dx, dot_y, 12.0, 12.0, 6.0, color);
            dx += 20.0;
        }
    }

    fn handle_input(&mut self, event: InputEvent) -> Transition {
        match self.step {
            Step::Name => match event {
                InputEvent::Char(ch) => {
                    if self.name.len() < 32 {
                        self.name.insert(self.cursor, ch);
                        self.cursor += ch.len_utf8();
                    }
                }
                InputEvent::Backspace => {
                    if self.cursor > 0 {
                        let prev = self.name[..self.cursor]
                            .char_indices()
                            .last()
                            .map(|(i, _)| i)
                            .unwrap_or(0);
                        self.name.remove(prev);
                        self.cursor = prev;
                    }
                }
                InputEvent::Enter | InputEvent::Tab => {
                    if !self.name.is_empty() {
                        self.step = Step::Interests;
                    }
                }
                InputEvent::Left => {
                    if self.cursor > 0 {
                        self.cursor = self.name[..self.cursor]
                            .char_indices()
                            .last()
                            .map(|(i, _)| i)
                            .unwrap_or(0);
                    }
                }
                InputEvent::Right => {
                    if self.cursor < self.name.len() {
                        self.cursor = self.name[self.cursor..]
                            .char_indices()
                            .nth(1)
                            .map(|(i, _)| self.cursor + i)
                            .unwrap_or(self.name.len());
                    }
                }
                _ => {}
            },
            Step::Interests => match event {
                InputEvent::Left => {
                    if self.interest_cursor > 0 {
                        self.interest_cursor -= 1;
                    }
                }
                InputEvent::Right => {
                    if self.interest_cursor < INTEREST_OPTIONS.len() - 1 {
                        self.interest_cursor += 1;
                    }
                }
                InputEvent::Up => {
                    if self.interest_cursor >= 5 {
                        self.interest_cursor -= 5;
                    }
                }
                InputEvent::Down => {
                    if self.interest_cursor + 5 < INTEREST_OPTIONS.len() {
                        self.interest_cursor += 5;
                    }
                }
                InputEvent::Enter | InputEvent::Char(' ') => {
                    self.selected_interests[self.interest_cursor] =
                        !self.selected_interests[self.interest_cursor];
                }
                InputEvent::Tab => {
                    if self.selected_interests.iter().any(|&s| s) {
                        self.step = Step::Finishing;
                    }
                }
                InputEvent::Escape => {
                    self.step = Step::Name;
                }
                _ => {}
            },
            Step::Finishing => {
                // No input during finishing animation
            }
        }
        Transition::None
    }
}
