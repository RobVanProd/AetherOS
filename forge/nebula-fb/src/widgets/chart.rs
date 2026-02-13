/// Chart widget — mini sparkline with anti-aliased lines.

use crate::renderer::Renderer;
use crate::theme;

/// Draw a sparkline chart within a given rectangle.
pub fn draw_sparkline(
    renderer: &mut Renderer,
    data: &[f64],
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    color: theme::Color,
) {
    if data.len() < 2 {
        return;
    }

    let max = data.iter().cloned().fold(1.0f64, f64::max);
    let min = data.iter().cloned().fold(0.0f64, f64::min);
    let range = (max - min).max(1.0);

    let step = w / (data.len() - 1) as f32;

    let points: Vec<(f32, f32)> = data
        .iter()
        .enumerate()
        .map(|(i, &v)| {
            let px = x + i as f32 * step;
            let py = y + h - ((v - min) as f32 / range as f32 * h);
            (px, py)
        })
        .collect();

    renderer.draw_polyline(&points, color, 1.5);
}
