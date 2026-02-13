/// Omnibar text input widget with cursor and placeholder.

use crate::renderer::Renderer;
use crate::text::TextRenderer;
use crate::theme;

pub struct TextInputState {
    pub text: String,
    pub cursor: usize,
    pub placeholder: String,
    pub focused: bool,
}

impl TextInputState {
    pub fn new(placeholder: &str) -> Self {
        Self {
            text: String::new(),
            cursor: 0,
            placeholder: placeholder.to_string(),
            focused: true,
        }
    }

    pub fn insert_char(&mut self, ch: char) {
        self.text.insert(self.cursor, ch);
        self.cursor += ch.len_utf8();
    }

    pub fn backspace(&mut self) {
        if self.cursor > 0 {
            let prev = self.text[..self.cursor]
                .char_indices()
                .last()
                .map(|(i, _)| i)
                .unwrap_or(0);
            self.text.remove(prev);
            self.cursor = prev;
        }
    }

    pub fn take_text(&mut self) -> String {
        let text = self.text.clone();
        self.text.clear();
        self.cursor = 0;
        text
    }

    pub fn move_left(&mut self) {
        if self.cursor > 0 {
            self.cursor = self.text[..self.cursor]
                .char_indices()
                .last()
                .map(|(i, _)| i)
                .unwrap_or(0);
        }
    }

    pub fn move_right(&mut self) {
        if self.cursor < self.text.len() {
            self.cursor = self.text[self.cursor..]
                .char_indices()
                .nth(1)
                .map(|(i, _)| self.cursor + i)
                .unwrap_or(self.text.len());
        }
    }
}

pub fn draw_omnibar(
    renderer: &mut Renderer,
    text_renderer: &TextRenderer,
    state: &TextInputState,
    width: u32,
    screen_height: u32,
) {
    let h = theme::OMNIBAR_HEIGHT as f32;
    let y = screen_height as f32 - h;

    // Background
    renderer.fill_rect(0.0, y, width as f32, h, theme::SURFACE);

    // Top border
    renderer.draw_line(0.0, y, width as f32, y, theme::CARD_BORDER, 1.0);

    let pad = 16.0;
    let text_y = y + (h - theme::FONT_SIZE_BODY) / 2.0;

    // Prompt indicator
    text_renderer.draw(renderer, ">", pad, text_y, theme::FONT_SIZE_BODY, theme::ACCENT_BLUE);
    let prompt_w = text_renderer.measure("> ", theme::FONT_SIZE_BODY);

    if state.text.is_empty() && !state.focused {
        // Placeholder
        text_renderer.draw(
            renderer,
            &state.placeholder,
            pad + prompt_w,
            text_y,
            theme::FONT_SIZE_BODY,
            theme::TEXT_MUTED,
        );
    } else if state.text.is_empty() {
        // Placeholder with blinking cursor
        text_renderer.draw(
            renderer,
            &state.placeholder,
            pad + prompt_w,
            text_y,
            theme::FONT_SIZE_BODY,
            theme::TEXT_MUTED,
        );
        // Cursor
        renderer.fill_rect(pad + prompt_w, text_y, 2.0, theme::FONT_SIZE_BODY, theme::ACCENT_BLUE);
    } else {
        // User text
        text_renderer.draw(
            renderer,
            &state.text,
            pad + prompt_w,
            text_y,
            theme::FONT_SIZE_BODY,
            theme::TEXT_PRIMARY,
        );
        // Cursor
        let cursor_x = pad + prompt_w + text_renderer.measure(&state.text[..state.cursor], theme::FONT_SIZE_BODY);
        renderer.fill_rect(cursor_x, text_y, 2.0, theme::FONT_SIZE_BODY, theme::ACCENT_BLUE);
    }

    // Enter icon on right
    let enter_text = "\u{23CE}";
    let enter_w = text_renderer.measure(enter_text, theme::FONT_SIZE_BODY);
    text_renderer.draw(
        renderer,
        enter_text,
        width as f32 - pad - enter_w,
        text_y,
        theme::FONT_SIZE_BODY,
        theme::TEXT_MUTED,
    );
}
