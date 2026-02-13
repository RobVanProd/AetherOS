/// 2D rendering wrapper around tiny-skia.

use tiny_skia::{
    FillRule, LineCap, Paint, PathBuilder, Pixmap, Stroke, Transform,
};

use crate::theme::Color;

pub struct Renderer {
    pub pixmap: Pixmap,
}

impl Renderer {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            pixmap: Pixmap::new(width, height).expect("create pixmap"),
        }
    }

    pub fn clear(&mut self, color: Color) {
        self.pixmap.fill(color.to_skia());
    }

    /// Copy pixmap data into a raw RGBA buffer.
    pub fn copy_to(&self, dst: &mut [u8]) {
        let src = self.pixmap.data();
        let len = dst.len().min(src.len());
        dst[..len].copy_from_slice(&src[..len]);
    }

    pub fn width(&self) -> u32 {
        self.pixmap.width()
    }

    pub fn height(&self) -> u32 {
        self.pixmap.height()
    }

    pub fn fill_rect(&mut self, x: f32, y: f32, w: f32, h: f32, color: Color) {
        let mut paint = Paint::default();
        paint.set_color(color.to_skia());
        paint.anti_alias = false;

        let rect = tiny_skia::Rect::from_xywh(x, y, w, h);
        if let Some(rect) = rect {
            self.pixmap.fill_rect(rect, &paint, Transform::identity(), None);
        }
    }

    pub fn fill_rounded_rect(&mut self, x: f32, y: f32, w: f32, h: f32, radius: f32, color: Color) {
        let mut paint = Paint::default();
        paint.set_color(color.to_skia());
        paint.anti_alias = true;

        if let Some(path) = rounded_rect_path(x, y, w, h, radius) {
            self.pixmap.fill_path(&path, &paint, FillRule::Winding, Transform::identity(), None);
        }
    }

    pub fn stroke_rounded_rect(&mut self, x: f32, y: f32, w: f32, h: f32, radius: f32, color: Color, width: f32) {
        let mut paint = Paint::default();
        paint.set_color(color.to_skia());
        paint.anti_alias = true;

        let mut stroke = Stroke::default();
        stroke.width = width;

        if let Some(path) = rounded_rect_path(x, y, w, h, radius) {
            self.pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
        }
    }

    pub fn draw_line(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, color: Color, width: f32) {
        let mut paint = Paint::default();
        paint.set_color(color.to_skia());
        paint.anti_alias = true;

        let mut stroke = Stroke::default();
        stroke.width = width;
        stroke.line_cap = LineCap::Round;

        let mut pb = PathBuilder::new();
        pb.move_to(x1, y1);
        pb.line_to(x2, y2);
        if let Some(path) = pb.finish() {
            self.pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
        }
    }

    /// Draw a polyline (for sparklines).
    pub fn draw_polyline(&mut self, points: &[(f32, f32)], color: Color, width: f32) {
        if points.len() < 2 {
            return;
        }
        let mut paint = Paint::default();
        paint.set_color(color.to_skia());
        paint.anti_alias = true;

        let mut stroke = Stroke::default();
        stroke.width = width;
        stroke.line_cap = LineCap::Round;

        let mut pb = PathBuilder::new();
        pb.move_to(points[0].0, points[0].1);
        for &(x, y) in &points[1..] {
            pb.line_to(x, y);
        }
        if let Some(path) = pb.finish() {
            self.pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
        }
    }

    /// Draw a filled pill (rounded capsule) for buttons.
    pub fn fill_pill(&mut self, x: f32, y: f32, w: f32, h: f32, color: Color) {
        self.fill_rounded_rect(x, y, w, h, h / 2.0, color);
    }

    /// Horizontal gradient rect.
    pub fn fill_gradient_h(&mut self, x: f32, y: f32, w: f32, h: f32, from: Color, to: Color) {
        // Approximate with thin vertical strips
        let steps = (w as u32).min(64);
        let strip_w = w / steps as f32;
        for i in 0..steps {
            let t = i as f32 / steps as f32;
            let c = from.blend(to, t);
            self.fill_rect(x + i as f32 * strip_w, y, strip_w + 1.0, h, c);
        }
    }
}

fn rounded_rect_path(x: f32, y: f32, w: f32, h: f32, r: f32) -> Option<tiny_skia::Path> {
    let r = r.min(w / 2.0).min(h / 2.0);
    let mut pb = PathBuilder::new();
    pb.move_to(x + r, y);
    pb.line_to(x + w - r, y);
    pb.quad_to(x + w, y, x + w, y + r);
    pb.line_to(x + w, y + h - r);
    pb.quad_to(x + w, y + h, x + w - r, y + h);
    pb.line_to(x + r, y + h);
    pb.quad_to(x, y + h, x, y + h - r);
    pb.line_to(x, y + r);
    pb.quad_to(x, y, x + r, y);
    pb.close();
    pb.finish()
}
