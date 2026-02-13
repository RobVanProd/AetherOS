/// Button widget — rounded pill with label, hover/selected state.

use crate::renderer::Renderer;
use crate::text::TextRenderer;
use crate::theme;

pub fn draw_button(
    renderer: &mut Renderer,
    text_renderer: &TextRenderer,
    label: &str,
    x: f32,
    y: f32,
    selected: bool,
) -> (f32, f32) {
    let pad_h = 12.0;
    let pad_v = 8.0;
    let text_w = text_renderer.measure(label, theme::FONT_SIZE_BODY);
    let w = text_w + pad_h * 2.0;
    let h = theme::FONT_SIZE_BODY + pad_v * 2.0;

    let (bg, fg) = if selected {
        (theme::ACCENT_BLUE, theme::BG)
    } else {
        (theme::CARD, theme::TEXT_PRIMARY)
    };

    renderer.fill_pill(x, y, w, h, bg);
    if !selected {
        renderer.stroke_rounded_rect(x, y, w, h, h / 2.0, theme::CARD_BORDER, 1.0);
    }
    text_renderer.draw(renderer, label, x + pad_h, y + pad_v, theme::FONT_SIZE_BODY, fg);

    (w, h)
}

/// Draw a chip (smaller, for tags/interests).
pub fn draw_chip(
    renderer: &mut Renderer,
    text_renderer: &TextRenderer,
    label: &str,
    x: f32,
    y: f32,
    selected: bool,
) -> (f32, f32) {
    let pad_h = 10.0;
    let pad_v = 5.0;
    let text_w = text_renderer.measure(label, theme::FONT_SIZE_SMALL);
    let w = text_w + pad_h * 2.0;
    let h = theme::FONT_SIZE_SMALL + pad_v * 2.0;

    let (bg, fg) = if selected {
        (theme::ACCENT_BLUE, theme::BG)
    } else {
        (theme::SURFACE, theme::TEXT_SECONDARY)
    };

    renderer.fill_pill(x, y, w, h, bg);
    if !selected {
        renderer.stroke_rounded_rect(x, y, w, h, h / 2.0, theme::CARD_BORDER, 1.0);
    }
    text_renderer.draw(renderer, label, x + pad_h, y + pad_v, theme::FONT_SIZE_SMALL, fg);

    (w, h)
}
