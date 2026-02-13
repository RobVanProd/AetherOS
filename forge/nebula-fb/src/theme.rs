/// AetherOS dark theme — GitHub-dark inspired.

#[derive(Clone, Copy)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }
    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }
    pub fn to_skia(self) -> tiny_skia::Color {
        tiny_skia::Color::from_rgba8(self.r, self.g, self.b, self.a)
    }
    pub fn blend(self, other: Color, t: f32) -> Color {
        let inv = 1.0 - t;
        Color {
            r: (self.r as f32 * inv + other.r as f32 * t) as u8,
            g: (self.g as f32 * inv + other.g as f32 * t) as u8,
            b: (self.b as f32 * inv + other.b as f32 * t) as u8,
            a: (self.a as f32 * inv + other.a as f32 * t) as u8,
        }
    }
}

// Background
pub const BG: Color = Color::rgb(0x0D, 0x11, 0x17);
pub const SURFACE: Color = Color::rgb(0x16, 0x1B, 0x22);
pub const CARD: Color = Color::rgb(0x1C, 0x21, 0x28);
pub const CARD_BORDER: Color = Color::rgb(0x30, 0x36, 0x3D);

// Text
pub const TEXT_PRIMARY: Color = Color::rgb(0xE6, 0xED, 0xF3);
pub const TEXT_SECONDARY: Color = Color::rgb(0x8B, 0x94, 0x9E);
pub const TEXT_MUTED: Color = Color::rgb(0x48, 0x4F, 0x58);

// Accents
pub const ACCENT_BLUE: Color = Color::rgb(0x58, 0xA6, 0xFF);
pub const ACCENT_GREEN: Color = Color::rgb(0x3F, 0xB9, 0x50);
pub const ACCENT_YELLOW: Color = Color::rgb(0xD2, 0x99, 0x22);
pub const ACCENT_RED: Color = Color::rgb(0xF8, 0x51, 0x49);

// Font sizes
pub const FONT_SIZE_TITLE: f32 = 32.0;
pub const FONT_SIZE_HEADING: f32 = 22.0;
pub const FONT_SIZE_BODY: f32 = 16.0;
pub const FONT_SIZE_SMALL: f32 = 13.0;
pub const FONT_SIZE_TINY: f32 = 11.0;

// Layout
pub const STATUS_BAR_HEIGHT: u32 = 40;
pub const OMNIBAR_HEIGHT: u32 = 48;
pub const CARD_RADIUS: f32 = 12.0;
pub const CARD_PADDING: u32 = 16;
pub const CARD_GAP: u32 = 16;
pub const CONTENT_MARGIN: u32 = 24;
pub const CARD_MIN_WIDTH: u32 = 350;
