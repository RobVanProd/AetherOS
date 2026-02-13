/// Status bar widget — top bar with AetherOS logo, time, CPU/Mem/Net indicators.

use crate::renderer::Renderer;
use crate::text::TextRenderer;
use crate::theme;

pub struct StatusBarData {
    pub cpu_pct: f64,
    pub mem_pct: f64,
    pub net_status: String,
    pub time_str: String,
}

pub fn draw_status_bar(
    renderer: &mut Renderer,
    text: &TextRenderer,
    data: &StatusBarData,
    width: u32,
) {
    let h = theme::STATUS_BAR_HEIGHT as f32;

    // Background
    renderer.fill_rect(0.0, 0.0, width as f32, h, theme::SURFACE);

    // Bottom border
    renderer.draw_line(0.0, h - 1.0, width as f32, h - 1.0, theme::CARD_BORDER, 1.0);

    let y = (h - theme::FONT_SIZE_SMALL) / 2.0;

    // AetherOS logo/text (left)
    text.draw(renderer, "\u{25CF}", 12.0, y, theme::FONT_SIZE_SMALL, theme::ACCENT_BLUE);
    text.draw(renderer, "AetherOS", 28.0, y, theme::FONT_SIZE_SMALL, theme::TEXT_PRIMARY);

    // Time (center)
    text.draw_centered(renderer, &data.time_str, 0.0, y, width as f32, theme::FONT_SIZE_SMALL, theme::TEXT_SECONDARY);

    // System indicators (right)
    let right_x = width as f32 - 12.0;

    // Net indicator
    let net_icon = if data.net_status.contains("10.") || data.net_status.contains("up") {
        "\u{25B2}"
    } else {
        "\u{25BC}"
    };
    let net_color = if data.net_status.contains("10.") || data.net_status.contains("up") {
        theme::ACCENT_GREEN
    } else {
        theme::ACCENT_RED
    };
    let net_w = text.measure("NET ", theme::FONT_SIZE_TINY);
    let net_icon_w = text.measure(net_icon, theme::FONT_SIZE_TINY);
    text.draw(renderer, "NET", right_x - net_w - net_icon_w, y + 1.0, theme::FONT_SIZE_TINY, theme::TEXT_MUTED);
    text.draw(renderer, net_icon, right_x - net_icon_w, y + 1.0, theme::FONT_SIZE_TINY, net_color);

    // Mem
    let mem_text = format!("Mem {:.0}%", data.mem_pct);
    let mem_w = text.measure(&mem_text, theme::FONT_SIZE_TINY);
    text.draw(renderer, &mem_text, right_x - net_w - net_icon_w - 16.0 - mem_w, y + 1.0, theme::FONT_SIZE_TINY, theme::TEXT_MUTED);

    // CPU
    let cpu_text = format!("CPU {:.0}%", data.cpu_pct);
    let cpu_w = text.measure(&cpu_text, theme::FONT_SIZE_TINY);
    text.draw(renderer, &cpu_text, right_x - net_w - net_icon_w - 16.0 - mem_w - 16.0 - cpu_w, y + 1.0, theme::FONT_SIZE_TINY, theme::TEXT_MUTED);
}
