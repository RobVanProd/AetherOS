/// Progress bar widget — animated, for setup wizard.

use crate::renderer::Renderer;
use crate::theme;

/// Draw an animated progress bar.
pub fn draw_progress_bar(
    renderer: &mut Renderer,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    progress: f32, // 0.0 to 1.0
) {
    let radius = h / 2.0;

    // Background track
    renderer.fill_rounded_rect(x, y, w, h, radius, theme::SURFACE);

    // Fill
    let fill_w = (w * progress.clamp(0.0, 1.0)).max(h); // min width = height for rounded caps
    if progress > 0.0 {
        renderer.fill_rounded_rect(x, y, fill_w, h, radius, theme::ACCENT_BLUE);
    }
}

/// Draw a progress bar with a shimmer/glow animation effect.
pub fn draw_progress_animated(
    renderer: &mut Renderer,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    progress: f32,
    time: f32, // elapsed time for animation
) {
    let radius = h / 2.0;

    // Background track
    renderer.fill_rounded_rect(x, y, w, h, radius, theme::SURFACE);

    // Fill
    let fill_w = (w * progress.clamp(0.0, 1.0)).max(h);
    if progress > 0.0 {
        renderer.fill_rounded_rect(x, y, fill_w, h, radius, theme::ACCENT_BLUE);

        // Shimmer stripe
        let shimmer_pos = ((time * 0.5) % 1.0) * fill_w;
        let shimmer_w = 30.0f32.min(fill_w * 0.3);
        if shimmer_pos + shimmer_w < fill_w {
            renderer.fill_rounded_rect(
                x + shimmer_pos,
                y,
                shimmer_w,
                h,
                radius,
                theme::Color::rgba(0xFF, 0xFF, 0xFF, 30),
            );
        }
    }
}
