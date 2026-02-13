/// Text rendering with fontdue — embedded Inter font, layout + render, word wrap.

use fontdue::{Font, FontSettings};

use crate::renderer::Renderer;
use crate::theme::Color;

static INTER_TTF: &[u8] = include_bytes!("../assets/Inter-Regular.ttf");

pub struct TextRenderer {
    font: Font,
}

impl TextRenderer {
    pub fn new() -> Self {
        let font = Font::from_bytes(INTER_TTF, FontSettings::default())
            .expect("load Inter font");
        Self { font }
    }

    /// Render a single line of text at (x, y) with the given size and color.
    /// Returns the width of the rendered text in pixels.
    pub fn draw(&self, renderer: &mut Renderer, text: &str, x: f32, y: f32, size: f32, color: Color) -> f32 {
        let pw = renderer.pixmap.width() as i32;
        let ph = renderer.pixmap.height() as i32;
        let mut cursor_x = x;
        for ch in text.chars() {
            let (metrics, bitmap) = self.font.rasterize(ch, size);
            if bitmap.is_empty() {
                cursor_x += metrics.advance_width;
                continue;
            }

            let gx = cursor_x as i32 + metrics.xmin;
            let gy = y as i32 + (size as i32 - metrics.height as i32 - metrics.ymin);

            // Blit glyph bitmap onto the pixmap
            let pm = renderer.pixmap.data_mut();

            for row in 0..metrics.height {
                for col in 0..metrics.width {
                    let alpha = bitmap[row * metrics.width + col];
                    if alpha == 0 {
                        continue;
                    }
                    let px = gx + col as i32;
                    let py = gy + row as i32;
                    if px < 0 || py < 0 || px >= pw || py >= ph {
                        continue;
                    }
                    let idx = (py as usize * pw as usize + px as usize) * 4;
                    if idx + 3 >= pm.len() {
                        continue;
                    }
                    let a = alpha as f32 / 255.0;
                    let inv = 1.0 - a;
                    pm[idx] = (pm[idx] as f32 * inv + color.r as f32 * a) as u8;
                    pm[idx + 1] = (pm[idx + 1] as f32 * inv + color.g as f32 * a) as u8;
                    pm[idx + 2] = (pm[idx + 2] as f32 * inv + color.b as f32 * a) as u8;
                    pm[idx + 3] = 255;
                }
            }

            cursor_x += metrics.advance_width;
        }
        cursor_x - x
    }

    /// Measure the width of text at a given font size.
    pub fn measure(&self, text: &str, size: f32) -> f32 {
        let mut w = 0.0f32;
        for ch in text.chars() {
            let (metrics, _) = self.font.rasterize(ch, size);
            w += metrics.advance_width;
        }
        w
    }

    /// Draw text centered horizontally within a given width.
    pub fn draw_centered(&self, renderer: &mut Renderer, text: &str, x: f32, y: f32, width: f32, size: f32, color: Color) {
        let tw = self.measure(text, size);
        let cx = x + (width - tw) / 2.0;
        self.draw(renderer, text, cx, y, size, color);
    }

    /// Word-wrap text to fit within max_width. Returns lines.
    pub fn wrap(&self, text: &str, size: f32, max_width: f32) -> Vec<String> {
        let mut lines = Vec::new();
        for paragraph in text.split('\n') {
            let words: Vec<&str> = paragraph.split_whitespace().collect();
            if words.is_empty() {
                lines.push(String::new());
                continue;
            }
            let mut current_line = String::new();
            for word in words {
                let test = if current_line.is_empty() {
                    word.to_string()
                } else {
                    format!("{} {}", current_line, word)
                };
                if self.measure(&test, size) <= max_width {
                    current_line = test;
                } else {
                    if !current_line.is_empty() {
                        lines.push(current_line);
                    }
                    current_line = word.to_string();
                }
            }
            if !current_line.is_empty() {
                lines.push(current_line);
            }
        }
        lines
    }

    /// Draw multi-line word-wrapped text. Returns total height used.
    pub fn draw_wrapped(
        &self,
        renderer: &mut Renderer,
        text: &str,
        x: f32,
        y: f32,
        max_width: f32,
        size: f32,
        line_height: f32,
        color: Color,
    ) -> f32 {
        let lines = self.wrap(text, size, max_width);
        for (i, line) in lines.iter().enumerate() {
            self.draw(renderer, line, x, y + i as f32 * line_height, size, color);
        }
        lines.len() as f32 * line_height
    }
}
