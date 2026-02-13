/// Card widget — rounded rect with title, body, optional metrics/progress bars.

use crate::renderer::Renderer;
use crate::text::TextRenderer;
use crate::theme;

/// Data for a card from the dashboard JSON.
#[derive(Clone, Debug, serde::Deserialize)]
pub struct CardData {
    #[serde(rename = "type")]
    pub card_type: String,
    pub title: String,
    #[serde(default)]
    pub body: Option<String>,
    #[serde(default)]
    pub metrics: Option<CardMetrics>,
    #[serde(default)]
    pub temp: Option<String>,
    #[serde(default)]
    pub desc: Option<String>,
    #[serde(default)]
    pub wind: Option<String>,
}

#[derive(Clone, Debug, serde::Deserialize)]
pub struct CardMetrics {
    #[serde(default)]
    pub cpu: f64,
    #[serde(default)]
    pub mem: f64,
}

/// Draw a card at the given position and size.
pub fn draw_card(
    renderer: &mut Renderer,
    text: &TextRenderer,
    data: &CardData,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    selected: bool,
) {
    let pad = theme::CARD_PADDING as f32;
    let radius = theme::CARD_RADIUS;

    // Card background
    renderer.fill_rounded_rect(x, y, w, h, radius, theme::CARD);

    // Border
    let border_color = if selected { theme::ACCENT_BLUE } else { theme::CARD_BORDER };
    renderer.stroke_rounded_rect(x, y, w, h, radius, border_color, if selected { 2.0 } else { 1.0 });

    // Title
    let title_color = match data.card_type.as_str() {
        "system" => theme::ACCENT_GREEN,
        "weather" => theme::ACCENT_BLUE,
        "alert" => theme::ACCENT_RED,
        "tip" => theme::ACCENT_YELLOW,
        _ => theme::TEXT_PRIMARY,
    };
    text.draw(renderer, &data.title, x + pad, y + pad, theme::FONT_SIZE_BODY, title_color);

    // Separator line
    let sep_y = y + pad + theme::FONT_SIZE_BODY + 6.0;
    renderer.draw_line(x + pad, sep_y, x + w - pad, sep_y, theme::CARD_BORDER, 1.0);

    let content_y = sep_y + 8.0;
    let content_w = w - pad * 2.0;

    match data.card_type.as_str() {
        "system" => {
            if let Some(ref metrics) = data.metrics {
                draw_metric_bar(renderer, text, "CPU", metrics.cpu, x + pad, content_y, content_w);
                draw_metric_bar(renderer, text, "Mem", metrics.mem, x + pad, content_y + 28.0, content_w);
            }
        }
        "weather" => {
            let mut cy = content_y;
            if let Some(ref temp) = data.temp {
                if let Some(ref desc) = data.desc {
                    text.draw(renderer, &format!("{}  {}", temp, desc), x + pad, cy, theme::FONT_SIZE_BODY, theme::TEXT_PRIMARY);
                    cy += 22.0;
                } else {
                    text.draw(renderer, temp, x + pad, cy, theme::FONT_SIZE_BODY, theme::TEXT_PRIMARY);
                    cy += 22.0;
                }
            }
            if let Some(ref wind) = data.wind {
                text.draw(renderer, &format!("Wind: {}", wind), x + pad, cy, theme::FONT_SIZE_SMALL, theme::TEXT_SECONDARY);
            }
        }
        _ => {
            if let Some(ref body) = data.body {
                text.draw_wrapped(
                    renderer,
                    body,
                    x + pad,
                    content_y,
                    content_w,
                    theme::FONT_SIZE_SMALL,
                    18.0,
                    theme::TEXT_SECONDARY,
                );
            }
        }
    }
}

fn draw_metric_bar(
    renderer: &mut Renderer,
    text_renderer: &TextRenderer,
    label: &str,
    value: f64,
    x: f32,
    y: f32,
    w: f32,
) {
    let label_w = 40.0;
    let bar_x = x + label_w;
    let bar_w = w - label_w - 50.0;
    let bar_h = 14.0;

    // Label
    text_renderer.draw(renderer, label, x, y, theme::FONT_SIZE_SMALL, theme::TEXT_SECONDARY);

    // Background bar
    renderer.fill_rounded_rect(bar_x, y + 2.0, bar_w, bar_h, 4.0, theme::SURFACE);

    // Fill bar
    let fill_w = (bar_w * value as f32 / 100.0).max(0.0).min(bar_w);
    let color = if value > 80.0 {
        theme::ACCENT_RED
    } else if value > 60.0 {
        theme::ACCENT_YELLOW
    } else {
        theme::ACCENT_GREEN
    };
    if fill_w > 0.0 {
        renderer.fill_rounded_rect(bar_x, y + 2.0, fill_w, bar_h, 4.0, color);
    }

    // Percentage text
    text_renderer.draw(
        renderer,
        &format!("{:.0}%", value),
        bar_x + bar_w + 6.0,
        y,
        theme::FONT_SIZE_SMALL,
        theme::TEXT_PRIMARY,
    );
}
