//! Color themes for the visualizer.

use ratatui::style::Color;

/// A color theme defines the gradient stops for spectrum bars and the line
/// color used for waveform/oscilloscope modes.
#[derive(Clone)]
pub struct Theme {
    pub name: &'static str,
    /// Colors from low to high amplitude. Must have at least one entry.
    pub gradient: &'static [Color],
    /// Line color for waveform mode.
    pub wave_color: Color,
    /// Line color for oscilloscope mode.
    pub scope_color: Color,
}

/// Convert a ratatui Color to (r, g, b). Named ANSI colors are mapped to
/// typical terminal defaults.
fn color_to_rgb(c: Color) -> (u8, u8, u8) {
    match c {
        Color::Rgb(r, g, b) => (r, g, b),
        Color::Black => (0, 0, 0),
        Color::Red => (205, 0, 0),
        Color::Green => (0, 205, 0),
        Color::Yellow => (205, 205, 0),
        Color::Blue => (0, 0, 238),
        Color::Magenta => (205, 0, 205),
        Color::Cyan => (0, 205, 205),
        Color::White => (255, 255, 255),
        Color::Gray => (128, 128, 128),
        _ => (255, 255, 255),
    }
}

/// Linearly interpolate between two colors. `t` is 0.0–1.0.
fn lerp_color(a: Color, b: Color, t: f32) -> Color {
    let (r0, g0, b0) = color_to_rgb(a);
    let (r1, g1, b1) = color_to_rgb(b);
    let t = t.clamp(0.0, 1.0);
    Color::Rgb(
        (r0 as f32 + (r1 as f32 - r0 as f32) * t) as u8,
        (g0 as f32 + (g1 as f32 - g0 as f32) * t) as u8,
        (b0 as f32 + (b1 as f32 - b0 as f32) * t) as u8,
    )
}

/// Sample a color from the gradient at position `v` (0.0–1.0), interpolating
/// between stops.
fn sample_gradient(gradient: &[Color], v: f32) -> Color {
    let v = v.clamp(0.0, 1.0);
    let last = (gradient.len() - 1) as f32;
    let pos = v * last;
    let lo = pos as usize;
    let hi = (lo + 1).min(gradient.len() - 1);
    let frac = pos - lo as f32;
    lerp_color(gradient[lo], gradient[hi], frac)
}

impl Theme {
    /// Pick a gradient color based on a normalized value (0.0–1.0),
    /// interpolating between gradient stops.
    pub fn bar_color(&self, normalized: f32) -> Color {
        sample_gradient(self.gradient, normalized)
    }
}

pub const THEMES: &[Theme] = &[
    Theme {
        name: "classic",
        gradient: &[
            Color::Rgb(0, 0, 180),
            Color::Rgb(0, 140, 220),
            Color::Rgb(0, 220, 180),
            Color::Rgb(0, 200, 80),
            Color::Rgb(160, 220, 0),
            Color::Rgb(255, 200, 0),
            Color::Rgb(255, 100, 0),
            Color::Rgb(220, 0, 0),
        ],
        wave_color: Color::Rgb(0, 220, 220),
        scope_color: Color::Rgb(0, 200, 80),
    },
    Theme {
        name: "fire",
        gradient: &[
            Color::Rgb(60, 0, 0),
            Color::Rgb(140, 0, 0),
            Color::Rgb(200, 40, 0),
            Color::Rgb(255, 100, 0),
            Color::Rgb(255, 160, 20),
            Color::Rgb(255, 210, 60),
            Color::Rgb(255, 240, 140),
            Color::Rgb(255, 255, 220),
        ],
        wave_color: Color::Rgb(255, 140, 20),
        scope_color: Color::Rgb(255, 220, 80),
    },
    Theme {
        name: "ocean",
        gradient: &[
            Color::Rgb(0, 10, 40),
            Color::Rgb(0, 30, 80),
            Color::Rgb(0, 70, 140),
            Color::Rgb(0, 120, 180),
            Color::Rgb(0, 170, 210),
            Color::Rgb(0, 210, 225),
            Color::Rgb(80, 235, 240),
            Color::Rgb(180, 255, 255),
        ],
        wave_color: Color::Rgb(0, 200, 220),
        scope_color: Color::Rgb(120, 240, 245),
    },
    Theme {
        name: "purple",
        gradient: &[
            Color::Rgb(20, 0, 40),
            Color::Rgb(50, 0, 90),
            Color::Rgb(90, 0, 150),
            Color::Rgb(140, 0, 200),
            Color::Rgb(180, 50, 235),
            Color::Rgb(210, 100, 255),
            Color::Rgb(235, 160, 255),
            Color::Rgb(255, 210, 255),
        ],
        wave_color: Color::Rgb(200, 80, 255),
        scope_color: Color::Rgb(240, 170, 255),
    },
    Theme {
        name: "matrix",
        gradient: &[
            Color::Rgb(0, 20, 0),
            Color::Rgb(0, 50, 0),
            Color::Rgb(0, 90, 0),
            Color::Rgb(0, 140, 0),
            Color::Rgb(0, 185, 0),
            Color::Rgb(0, 225, 0),
            Color::Rgb(60, 245, 60),
            Color::Rgb(160, 255, 160),
        ],
        wave_color: Color::Rgb(0, 210, 0),
        scope_color: Color::Rgb(80, 255, 80),
    },
    Theme {
        name: "synthwave",
        gradient: &[
            Color::Rgb(10, 0, 30),
            Color::Rgb(40, 0, 90),
            Color::Rgb(80, 0, 150),
            Color::Rgb(140, 0, 180),
            Color::Rgb(200, 0, 170),
            Color::Rgb(255, 20, 147),
            Color::Rgb(255, 80, 80),
            Color::Rgb(255, 150, 50),
        ],
        wave_color: Color::Rgb(255, 20, 147),
        scope_color: Color::Rgb(200, 0, 180),
    },
    Theme {
        name: "tokyo-night-moon",
        gradient: &[
            Color::Rgb(0x3e, 0x68, 0xd7), // blue0
            Color::Rgb(0x82, 0xaa, 0xff), // blue
            Color::Rgb(0x65, 0xbc, 0xff), // blue1
            Color::Rgb(0x86, 0xe1, 0xfc), // cyan
            Color::Rgb(0xc3, 0xe8, 0x8d), // green
            Color::Rgb(0xff, 0xc7, 0x77), // yellow
            Color::Rgb(0xff, 0x96, 0x6c), // orange
            Color::Rgb(0xff, 0x75, 0x7f), // red
        ],
        wave_color: Color::Rgb(0x82, 0xaa, 0xff),    // blue
        scope_color: Color::Rgb(0x86, 0xe1, 0xfc),   // cyan
    },
    Theme {
        name: "mono",
        gradient: &[
            Color::Rgb(50, 50, 50),
            Color::Rgb(85, 85, 85),
            Color::Rgb(120, 120, 120),
            Color::Rgb(150, 150, 150),
            Color::Rgb(180, 180, 180),
            Color::Rgb(205, 205, 205),
            Color::Rgb(230, 230, 230),
            Color::White,
        ],
        wave_color: Color::Rgb(200, 200, 200),
        scope_color: Color::White,
    },
];
