//! Color System
//!
//! Adaptive colors that respond to time of day, content, and user preference.
//! Not just "light mode" and "dark mode"—a continuous spectrum.

use crate::render::Color;

/// Time of day for adaptive theming
#[derive(Clone, Copy, Debug)]
pub enum TimeOfDay {
    Dawn,    // 5-7am
    Morning, // 7-12pm
    Day,     // 12-5pm
    Evening, // 5-8pm
    Dusk,    // 8-10pm
    Night,   // 10pm-5am
}

impl TimeOfDay {
    pub fn from_hour(hour: u32) -> Self {
        match hour {
            5..=6 => Self::Dawn,
            7..=11 => Self::Morning,
            12..=16 => Self::Day,
            17..=19 => Self::Evening,
            20..=21 => Self::Dusk,
            _ => Self::Night,
        }
    }
}

/// A complete color palette
#[derive(Clone, Debug)]
pub struct Palette {
    pub background: Color,
    pub surface: Color,
    pub surface_elevated: Color,
    pub text: Color,
    pub text_secondary: Color,
    pub accent: Color,
    pub accent_dim: Color,
    pub success: Color,
    pub warning: Color,
    pub error: Color,
}

impl Palette {
    /// Default dark palette (night)
    pub fn dark() -> Self {
        Self {
            background: Color::rgb(0.02, 0.02, 0.04),
            surface: Color::rgb(0.08, 0.08, 0.12),
            surface_elevated: Color::rgb(0.12, 0.12, 0.16),
            text: Color::rgb(0.92, 0.92, 0.94),
            text_secondary: Color::rgba(0.92, 0.92, 0.94, 0.6),
            accent: Color::rgb(0.4, 0.6, 1.0),
            accent_dim: Color::rgba(0.4, 0.6, 1.0, 0.3),
            success: Color::rgb(0.3, 0.8, 0.5),
            warning: Color::rgb(0.9, 0.7, 0.2),
            error: Color::rgb(0.9, 0.3, 0.3),
        }
    }

    /// Light palette (day)
    pub fn light() -> Self {
        Self {
            background: Color::rgb(0.96, 0.96, 0.98),
            surface: Color::rgb(1.0, 1.0, 1.0),
            surface_elevated: Color::rgb(0.98, 0.98, 1.0),
            text: Color::rgb(0.1, 0.1, 0.12),
            text_secondary: Color::rgba(0.1, 0.1, 0.12, 0.6),
            accent: Color::rgb(0.2, 0.4, 0.9),
            accent_dim: Color::rgba(0.2, 0.4, 0.9, 0.2),
            success: Color::rgb(0.2, 0.6, 0.4),
            warning: Color::rgb(0.8, 0.6, 0.1),
            error: Color::rgb(0.8, 0.2, 0.2),
        }
    }

    /// Dawn palette (warm, gentle)
    pub fn dawn() -> Self {
        Self {
            background: Color::rgb(0.12, 0.08, 0.08),
            surface: Color::rgb(0.16, 0.12, 0.12),
            surface_elevated: Color::rgb(0.20, 0.16, 0.14),
            text: Color::rgb(0.95, 0.90, 0.88),
            text_secondary: Color::rgba(0.95, 0.90, 0.88, 0.6),
            accent: Color::rgb(1.0, 0.6, 0.4),
            accent_dim: Color::rgba(1.0, 0.6, 0.4, 0.3),
            success: Color::rgb(0.4, 0.7, 0.5),
            warning: Color::rgb(0.9, 0.7, 0.3),
            error: Color::rgb(0.9, 0.4, 0.3),
        }
    }

    /// Dusk palette (cool, calming)
    pub fn dusk() -> Self {
        Self {
            background: Color::rgb(0.06, 0.06, 0.10),
            surface: Color::rgb(0.10, 0.10, 0.16),
            surface_elevated: Color::rgb(0.14, 0.14, 0.20),
            text: Color::rgb(0.88, 0.88, 0.94),
            text_secondary: Color::rgba(0.88, 0.88, 0.94, 0.6),
            accent: Color::rgb(0.6, 0.5, 0.9),
            accent_dim: Color::rgba(0.6, 0.5, 0.9, 0.3),
            success: Color::rgb(0.4, 0.7, 0.6),
            warning: Color::rgb(0.9, 0.6, 0.4),
            error: Color::rgb(0.9, 0.4, 0.5),
        }
    }

    /// Get palette for time of day
    pub fn for_time(time: TimeOfDay) -> Self {
        match time {
            TimeOfDay::Dawn => Self::dawn(),
            TimeOfDay::Morning => Self::lerp(&Self::dawn(), &Self::light(), 0.5),
            TimeOfDay::Day => Self::light(),
            TimeOfDay::Evening => Self::lerp(&Self::light(), &Self::dusk(), 0.5),
            TimeOfDay::Dusk => Self::dusk(),
            TimeOfDay::Night => Self::dark(),
        }
    }

    /// Interpolate between two palettes
    pub fn lerp(a: &Self, b: &Self, t: f32) -> Self {
        Self {
            background: lerp_color(a.background, b.background, t),
            surface: lerp_color(a.surface, b.surface, t),
            surface_elevated: lerp_color(a.surface_elevated, b.surface_elevated, t),
            text: lerp_color(a.text, b.text, t),
            text_secondary: lerp_color(a.text_secondary, b.text_secondary, t),
            accent: lerp_color(a.accent, b.accent, t),
            accent_dim: lerp_color(a.accent_dim, b.accent_dim, t),
            success: lerp_color(a.success, b.success, t),
            warning: lerp_color(a.warning, b.warning, t),
            error: lerp_color(a.error, b.error, t),
        }
    }
}

fn lerp_color(a: Color, b: Color, t: f32) -> Color {
    Color::rgba(
        a.r + (b.r - a.r) * t,
        a.g + (b.g - a.g) * t,
        a.b + (b.b - a.b) * t,
        a.a + (b.a - a.a) * t,
    )
}

/// User accent color presets
#[derive(Clone, Copy, Debug)]
pub enum AccentPreset {
    Blue,
    Purple,
    Pink,
    Red,
    Orange,
    Yellow,
    Green,
    Teal,
    Cyan,
}

impl AccentPreset {
    pub fn color(&self) -> Color {
        match self {
            Self::Blue => Color::rgb(0.4, 0.6, 1.0),
            Self::Purple => Color::rgb(0.6, 0.4, 1.0),
            Self::Pink => Color::rgb(1.0, 0.4, 0.7),
            Self::Red => Color::rgb(1.0, 0.4, 0.4),
            Self::Orange => Color::rgb(1.0, 0.6, 0.3),
            Self::Yellow => Color::rgb(1.0, 0.85, 0.3),
            Self::Green => Color::rgb(0.4, 0.9, 0.5),
            Self::Teal => Color::rgb(0.3, 0.85, 0.8),
            Self::Cyan => Color::rgb(0.3, 0.8, 1.0),
        }
    }
}
