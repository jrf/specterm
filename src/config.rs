//! Configuration file support. Loads/saves from ~/.config/specterm/config.toml.

use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub mode: String,
    pub theme: String,
    pub theme_catalog: String,
    pub fps: u64,
    pub low_freq: f32,
    pub high_freq: f32,
    pub smoothing: f32,
    pub monstercat: bool,
    pub noise_floor: f32,
    /// Color bars by position (true) or amplitude (false).
    pub gradient_by_position: bool,
    /// Width of each bar in terminal columns.
    pub bar_width: usize,
    /// Spacing between bars in terminal columns.
    pub bar_spacing: usize,
    /// Sensitivity in percent (100 = normal).
    pub sensitivity: u32,
    /// Per-band equalizer gains (evenly distributed across frequency range).
    pub eq: Vec<f32>,
    /// Allow non-theme in-app settings changes to be written back to this config file.
    /// Theme picker changes are always session-only.
    pub save_settings: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            mode: "spectrum".to_string(),
            theme: String::new(),
            theme_catalog: String::new(),
            fps: 60,
            low_freq: 50.0,
            high_freq: 10000.0,
            smoothing: 0.3,
            monstercat: true,
            noise_floor: 0.0,
            gradient_by_position: false,
            bar_width: 2,
            bar_spacing: 1,
            sensitivity: 100,
            eq: vec![1.0, 1.0, 1.0, 1.0, 1.0],
            save_settings: false,
        }
    }
}

/// Valid ranges for settings. The in-app settings overlay uses the same bounds.
pub const SMOOTHING_RANGE: (f32, f32) = (0.0, 0.99);
pub const NOISE_FLOOR_RANGE: (f32, f32) = (0.0, 0.05);
pub const BAR_WIDTH_RANGE: (usize, usize) = (1, 8);
pub const BAR_SPACING_RANGE: (usize, usize) = (0, 4);
pub const SENSITIVITY_RANGE: (u32, u32) = (10, 500);
pub const EQ_GAIN_RANGE: (f32, f32) = (0.0, 3.0);
pub const FPS_RANGE: (u64, u64) = (1, 240);
/// Lowest allowed `low_freq`, and the minimum width of the frequency range.
const MIN_FREQ: f32 = 1.0;
const MIN_FREQ_RATIO: f32 = 2.0;

fn clamp_f32(v: f32, (lo, hi): (f32, f32), default: f32) -> f32 {
    if v.is_finite() {
        v.clamp(lo, hi)
    } else {
        default
    }
}

impl Config {
    /// Return a copy with every field in a usable range, plus a description of
    /// each change made. Values from the config file and CLI pass through here
    /// once, so the rest of the program can rely on them.
    ///
    /// `sample_rate` caps `high_freq` at Nyquist.
    pub fn validated(&self, sample_rate: u32) -> (Config, Vec<String>) {
        let d = Config::default();
        let mut c = self.clone();
        let mut notes = Vec::new();
        let mut note = |field: &str, from: String, to: String| {
            if from != to {
                notes.push(format!("{field}: {from} -> {to}"));
            }
        };

        let fps = c.fps.clamp(FPS_RANGE.0, FPS_RANGE.1);
        note("fps", c.fps.to_string(), fps.to_string());
        c.fps = fps;

        let smoothing = clamp_f32(c.smoothing, SMOOTHING_RANGE, d.smoothing);
        note("smoothing", c.smoothing.to_string(), smoothing.to_string());
        c.smoothing = smoothing;

        let noise = clamp_f32(c.noise_floor, NOISE_FLOOR_RANGE, d.noise_floor);
        note("noise_floor", c.noise_floor.to_string(), noise.to_string());
        c.noise_floor = noise;

        let bw = c.bar_width.clamp(BAR_WIDTH_RANGE.0, BAR_WIDTH_RANGE.1);
        note("bar_width", c.bar_width.to_string(), bw.to_string());
        c.bar_width = bw;

        let bs = c
            .bar_spacing
            .clamp(BAR_SPACING_RANGE.0, BAR_SPACING_RANGE.1);
        note("bar_spacing", c.bar_spacing.to_string(), bs.to_string());
        c.bar_spacing = bs;

        let sens = c
            .sensitivity
            .clamp(SENSITIVITY_RANGE.0, SENSITIVITY_RANGE.1);
        note("sensitivity", c.sensitivity.to_string(), sens.to_string());
        c.sensitivity = sens;

        if c.eq.is_empty() {
            note("eq", "[]".into(), format!("{:?}", d.eq));
            c.eq = d.eq.clone();
        }
        for (i, g) in c.eq.iter_mut().enumerate() {
            let v = clamp_f32(*g, EQ_GAIN_RANGE, 1.0);
            note(&format!("eq[{i}]"), g.to_string(), v.to_string());
            *g = v;
        }

        // Frequency range: both finite, low >= 1 Hz, high <= Nyquist, and
        // high at least an octave above low. Anything else falls back to the
        // defaults rather than guessing which bound the user meant.
        let nyquist = sample_rate as f32 / 2.0;
        let (lo, hi) = (c.low_freq, c.high_freq);
        let usable = lo.is_finite() && hi.is_finite() && lo >= MIN_FREQ && hi > lo;
        let (mut new_lo, mut new_hi) = if usable {
            (lo, hi)
        } else {
            (d.low_freq, d.high_freq)
        };
        new_hi = new_hi.min(nyquist);
        if new_hi < new_lo * MIN_FREQ_RATIO {
            new_lo = d.low_freq;
            new_hi = d.high_freq.min(nyquist);
        }
        note("low_freq", lo.to_string(), new_lo.to_string());
        note("high_freq", hi.to_string(), new_hi.to_string());
        c.low_freq = new_lo;
        c.high_freq = new_hi;

        (c, notes)
    }
}

/// Get the config file path (~/.config/specterm/config.toml).
pub fn config_path() -> PathBuf {
    dirs().join("config.toml")
}

fn dirs() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".config").join("specterm")
}

/// Load config from disk, falling back to defaults for missing fields.
pub fn load() -> Config {
    let path = config_path();
    match fs::read_to_string(&path) {
        Ok(contents) => toml::from_str(&contents).unwrap_or_default(),
        Err(_) => Config::default(),
    }
}

/// Save config to disk with descriptive comments.
pub fn save(config: &Config) -> std::io::Result<()> {
    // Round floats to avoid long decimal strings like 0.35000002
    let mut rounded = config.clone();
    rounded.smoothing = (rounded.smoothing * 100.0).round() / 100.0;
    rounded.noise_floor = (rounded.noise_floor * 10000.0).round() / 10000.0;
    rounded.low_freq = rounded.low_freq.round();
    rounded.high_freq = rounded.high_freq.round();

    let dir = dirs();
    fs::create_dir_all(&dir)?;
    let raw = toml::to_string_pretty(&rounded).unwrap_or_default();
    let commented = add_comments(&raw);
    fs::write(config_path(), commented)
}

/// Insert descriptive comments above known config keys.
fn add_comments(toml: &str) -> String {
    let comments: &[(&str, &str)] = &[
        ("mode =", "# Visualization mode: spectrum, wave, scope, stereo"),
        ("theme =", "# Explicit path to the selected theme file"),
        ("theme_catalog =", "# Explicit path to the picker catalog"),
        ("fps =", "# Target frames per second"),

        ("low_freq =", "# Low frequency cutoff in Hz"),
        ("high_freq =", "# High frequency cutoff in Hz"),
        ("smoothing =", "# Temporal smoothing factor (0.0 = none, 0.99 = heavy)"),
        ("monstercat =", "# Monstercat-style smoothing (connects bar tops in a smooth curve)"),
        ("noise_floor =", "# Noise gate threshold (bars below this are zeroed)"),
        ("gradient_by_position =", "# Color by position (true) or amplitude (false)"),
        ("bar_width =", "# Width of each bar in terminal columns (1-8)"),
        ("bar_spacing =", "# Gap between bars in terminal columns (0-4)"),
        ("sensitivity =", "# Sensitivity in percent (100 = normal, higher = louder)"),
        ("eq =", "# Per-band equalizer gains (evenly distributed across frequency range).\n# Default [1.0, 1.0, 1.0, 1.0, 1.0] = flat. Values > 1.0 boost, < 1.0 cut."),
        ("save_settings =", "# Allow non-theme in-app settings changes to overwrite this file (default false)"),
    ];

    let mut result = String::with_capacity(toml.len() * 2);
    for line in toml.lines() {
        let trimmed = line.trim();
        for &(key, comment) in comments {
            if trimmed.starts_with(key) {
                result.push_str(comment);
                result.push('\n');
                break;
            }
        }
        result.push_str(line);
        result.push('\n');
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    const SR: u32 = 48_000;

    #[test]
    fn defaults_are_already_valid() {
        let (c, notes) = Config::default().validated(SR);
        assert!(notes.is_empty(), "{notes:?}");
        assert_eq!(c.low_freq, 50.0);
        assert_eq!(c.high_freq, 10_000.0);
    }

    #[test]
    fn out_of_range_values_are_clamped_with_notes() {
        let c = Config {
            fps: 0,
            smoothing: 1.5,
            noise_floor: -1.0,
            bar_width: 20,
            bar_spacing: 9,
            sensitivity: 1,
            eq: vec![5.0, -1.0, 1.2],
            ..Config::default()
        };
        let (v, notes) = c.validated(SR);
        assert_eq!(v.fps, 1);
        assert_eq!(v.smoothing, 0.99);
        assert_eq!(v.noise_floor, 0.0);
        assert_eq!(v.bar_width, 8);
        assert_eq!(v.bar_spacing, 4);
        assert_eq!(v.sensitivity, 10);
        assert_eq!(v.eq, vec![3.0, 0.0, 1.2]);
        assert_eq!(notes.len(), 8, "{notes:?}");
    }

    #[test]
    fn non_finite_floats_fall_back_to_defaults() {
        let c = Config {
            smoothing: f32::NAN,
            noise_floor: f32::INFINITY,
            eq: vec![f32::NAN],
            ..Config::default()
        };
        let (v, _) = c.validated(SR);
        assert_eq!(v.smoothing, 0.3);
        assert_eq!(v.noise_floor, 0.0);
        assert_eq!(v.eq, vec![1.0]);
    }

    #[test]
    fn empty_eq_falls_back_to_flat() {
        let c = Config {
            eq: vec![],
            ..Config::default()
        };
        assert_eq!(c.validated(SR).0.eq, vec![1.0; 5]);
    }

    #[test]
    fn unusable_frequency_ranges_fall_back_to_defaults() {
        for (lo, hi) in [
            (5000.0, 100.0),    // inverted
            (1000.0, 1000.0),   // empty
            (50.0, 0.0),        // zero high (was: blank display)
            (50.0, -5.0),       // negative (was: NaN bars)
            (f32::NAN, 1000.0), // non-finite
            (0.0, 1000.0),      // below 1 Hz
            (8000.0, 10000.0),  // narrower than an octave
        ] {
            let c = Config {
                low_freq: lo,
                high_freq: hi,
                ..Config::default()
            };
            let (v, notes) = c.validated(SR);
            assert_eq!(
                (v.low_freq, v.high_freq),
                (50.0, 10_000.0),
                "input ({lo}, {hi})"
            );
            assert!(!notes.is_empty(), "no note for ({lo}, {hi})");
        }
    }

    #[test]
    fn high_freq_is_capped_at_nyquist_but_valid_ranges_are_kept() {
        let c = Config {
            low_freq: 30.0,
            high_freq: 30_000.0,
            ..Config::default()
        };
        let (v, _) = c.validated(SR);
        assert_eq!((v.low_freq, v.high_freq), (30.0, 24_000.0));

        let c = Config {
            low_freq: 100.0,
            high_freq: 5000.0,
            ..Config::default()
        };
        let (v, notes) = c.validated(SR);
        assert_eq!((v.low_freq, v.high_freq), (100.0, 5000.0));
        assert!(notes.is_empty());
    }

    #[test]
    fn saved_config_round_trips_and_parses() {
        let c = Config::default();
        let raw = toml::to_string_pretty(&c).unwrap();
        let commented = add_comments(&raw);
        let back: Config = toml::from_str(&commented).unwrap();
        assert_eq!(back.eq, c.eq);
        assert_eq!(back.low_freq, c.low_freq);
        assert_eq!(back.mode, c.mode);
    }
}
