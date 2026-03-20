//! Color themes for the visualizer.
//!
//! Themes are loaded from TOML files in `~/.config/termwave/themes/`. Each file
//! defines a `[colors]` table of named hex colors and a `[visualizer]` table
//! that references those names for the gradient, wave, and scope colors.

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use ratatui::style::Color;

/// A color theme defines the gradient stops for spectrum bars and the line
/// color used for waveform/oscilloscope modes.
#[derive(Clone)]
pub struct Theme {
    pub name: String,
    /// Colors from low to high amplitude. Must have at least one entry.
    pub gradient: Vec<Color>,
    /// Line color for waveform mode.
    pub wave_color: Color,
    /// Line color for oscilloscope mode.
    pub scope_color: Color,
}

/// Convert a ratatui Color to (r, g, b).
fn color_to_rgb(c: Color) -> (u8, u8, u8) {
    match c {
        Color::Rgb(r, g, b) => (r, g, b),
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
        sample_gradient(&self.gradient, normalized)
    }
}

// ---------------------------------------------------------------------------
// Theme loading from TOML files
// ---------------------------------------------------------------------------

/// Parse a hex color string like "#82aaff" into a ratatui Color.
fn parse_hex(s: &str) -> Option<Color> {
    let s = s.strip_prefix('#')?;
    if s.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&s[0..2], 16).ok()?;
    let g = u8::from_str_radix(&s[2..4], 16).ok()?;
    let b = u8::from_str_radix(&s[4..6], 16).ok()?;
    Some(Color::Rgb(r, g, b))
}

/// Resolve a color value — either a direct hex string or a reference to a
/// named color in the palette.
fn resolve_color(value: &str, palette: &HashMap<String, Color>) -> Option<Color> {
    if value.starts_with('#') {
        parse_hex(value)
    } else {
        palette.get(value).copied()
    }
}

/// Load a single theme from a TOML string. The file name (without extension)
/// is used as the theme name.
fn parse_theme(name: &str, content: &str) -> Option<Theme> {
    let table: toml::Table = content.parse().ok()?;

    // Parse [colors] into a palette
    let colors_table = table.get("colors")?.as_table()?;
    let mut palette = HashMap::new();
    for (key, val) in colors_table {
        if let Some(hex) = val.as_str().and_then(parse_hex) {
            palette.insert(key.clone(), hex);
        }
    }

    // Parse [visualizer]
    let vis = table.get("visualizer")?.as_table()?;

    let gradient_arr = vis.get("gradient")?.as_array()?;
    let gradient: Vec<Color> = gradient_arr
        .iter()
        .filter_map(|v| v.as_str().and_then(|s| resolve_color(s, &palette)))
        .collect();

    if gradient.is_empty() {
        return None;
    }

    let wave_color = vis
        .get("wave_color")
        .and_then(|v| v.as_str())
        .and_then(|s| resolve_color(s, &palette))
        .unwrap_or(gradient[gradient.len() / 2]);

    let scope_color = vis
        .get("scope_color")
        .and_then(|v| v.as_str())
        .and_then(|s| resolve_color(s, &palette))
        .unwrap_or(gradient[gradient.len() / 2]);

    Some(Theme {
        name: name.to_string(),
        gradient,
        wave_color,
        scope_color,
    })
}

/// Get the themes directory path (~/.config/termwave/themes/).
fn themes_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home)
        .join(".config")
        .join("termwave")
        .join("themes")
}

/// Load all themes from the themes directory. Returns an empty vec if no
/// valid theme files are found.
pub fn load_themes() -> Vec<Theme> {
    let dir = themes_dir();
    let mut themes = Vec::new();

    if let Ok(entries) = fs::read_dir(&dir) {
        let mut entries: Vec<_> = entries.filter_map(|e| e.ok()).collect();
        entries.sort_by_key(|e| e.file_name());

        for entry in entries {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("toml") {
                let name = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("unknown")
                    .to_string();
                if let Ok(content) = fs::read_to_string(&path) {
                    if let Some(theme) = parse_theme(&name, &content) {
                        themes.push(theme);
                    }
                }
            }
        }
    }

    themes
}
