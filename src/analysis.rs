//! DSP: FFT spectrum analysis, waveform extraction, and smoothing.

use rustfft::{num_complex::Complex, Fft, FftPlanner};

/// Large FFT window for bass frequencies (below ~200 Hz). 4096 at 48 kHz gives
/// ~11.7 Hz bin spacing — sufficient for resolving bars down to 50 Hz (our default
/// low_freq). Halved from 8192 to cut bass latency from ~170ms to ~85ms.
pub const FFT_SIZE: usize = 4096;

/// FFT output size for mids/highs. We take only `FFT_WINDOW_SMALL` real samples
/// and zero-pad to this size, giving ~42ms time resolution with the frequency
/// bin spacing of a 4096-point FFT.
pub const FFT_SIZE_SMALL: usize = 4096;

/// Number of real samples to use for the small FFT before zero-padding.
const FFT_WINDOW_SMALL: usize = 2048;

/// Frequency below which we use the large FFT for better bass resolution.
const BASS_CROSSOVER_HZ: f32 = 200.0;

/// Cached FFT plans for reuse across frames. Holds both a large plan (for bass)
/// and a small plan (for mids/highs) to balance frequency vs time resolution.
pub struct SpectrumAnalyzer {
    fft_large: std::sync::Arc<dyn Fft<f32>>,
    fft_small: std::sync::Arc<dyn Fft<f32>>,
}

impl SpectrumAnalyzer {
    pub fn new() -> Self {
        let mut planner = FftPlanner::new();
        let fft_large = planner.plan_fft_forward(FFT_SIZE);
        let fft_small = planner.plan_fft_forward(FFT_SIZE_SMALL);
        Self {
            fft_large,
            fft_small,
        }
    }

    /// Run an FFT and return positive-frequency magnitudes.
    ///
    /// `window` controls how many real samples are used (the rest is zero-padded
    /// to `fft_size`). This lets a short analysis window produce the same number
    /// of frequency bins as a larger FFT.
    fn run_fft(
        fft: &dyn Fft<f32>,
        samples: &[f32],
        window: usize,
        fft_size: usize,
    ) -> Vec<f32> {
        let mut buffer: Vec<Complex<f32>> = samples
            .iter()
            .take(window)
            .enumerate()
            .map(|(i, &s)| {
                // Hann window: tapers edges to zero, reducing spectral leakage
                let w = 0.5 * (1.0 - (2.0 * std::f32::consts::PI * i as f32 / (window - 1) as f32).cos());
                Complex { re: s * w, im: 0.0 }
            })
            .collect();
        buffer.resize(fft_size, Complex { re: 0.0, im: 0.0 });
        fft.process(&mut buffer);
        buffer[..fft_size / 2]
            .iter()
            .map(|c| c.norm() / fft_size as f32)
            .collect()
    }

    /// Compute dual-resolution magnitude spectra: a large FFT for bass and a
    /// smaller FFT (short window, zero-padded) for mids/highs.
    /// Returns `(bass_magnitudes, main_magnitudes)`.
    pub fn spectrum_dual(&self, samples: &[f32]) -> (Vec<f32>, Vec<f32>) {
        let bass = Self::run_fft(&*self.fft_large, samples, FFT_SIZE, FFT_SIZE);
        // Use only the most recent FFT_WINDOW_SMALL samples, zero-padded to FFT_SIZE_SMALL
        let offset = samples.len().saturating_sub(FFT_WINDOW_SMALL);
        let main = Self::run_fft(&*self.fft_small, &samples[offset..], FFT_WINDOW_SMALL, FFT_SIZE_SMALL);
        (bass, main)
    }
}

/// Bin a dual-resolution spectrum into `n` bars using logarithmic frequency scaling.
///
/// `bass_magnitudes` comes from the large FFT (better frequency resolution for bass),
/// `main_magnitudes` comes from the small FFT (better time resolution for mids/highs).
/// Bars whose center frequency is below `BASS_CROSSOVER_HZ` use the bass magnitudes;
/// the rest use the main magnitudes.
///
/// `low_freq` and `high_freq` control the visible frequency range.
pub fn bin_spectrum(
    bass_magnitudes: &[f32],
    main_magnitudes: &[f32],
    n: usize,
    sample_rate: u32,
    low_freq: f32,
    high_freq: f32,
) -> Vec<f32> {
    if (bass_magnitudes.is_empty() && main_magnitudes.is_empty()) || n == 0 {
        return vec![0.0; n];
    }

    let nyquist = sample_rate as f32 / 2.0;
    let bass_freq_per_bin = nyquist / bass_magnitudes.len().max(1) as f32;
    let main_freq_per_bin = nyquist / main_magnitudes.len().max(1) as f32;

    let min_freq = low_freq.max(1.0);
    let max_freq = high_freq.min(nyquist);
    let log_min = min_freq.ln();
    let log_max = max_freq.ln();

    let mut bars = vec![0.0f32; n];

    // Anti-clumping: track the last used bin index per magnitude source so
    // adjacent bars that map to the same FFT bin get pushed apart. Without
    // this, the lowest bass bars all read the same bin and move in lockstep.
    let mut prev_bass_lo: usize = 0;
    let mut prev_main_lo: usize = 0;

    #[allow(clippy::needless_range_loop)]
    for i in 0..n {
        let freq_lo = (log_min + (log_max - log_min) * i as f32 / n as f32).exp();
        let freq_hi = (log_min + (log_max - log_min) * (i + 1) as f32 / n as f32).exp();
        let center_freq = (freq_lo + freq_hi) * 0.5;

        // Pick the appropriate magnitude source based on center frequency
        let (magnitudes, freq_per_bin, prev_lo) = if center_freq < BASS_CROSSOVER_HZ
            && !bass_magnitudes.is_empty()
        {
            (bass_magnitudes, bass_freq_per_bin, &mut prev_bass_lo)
        } else if !main_magnitudes.is_empty() {
            (main_magnitudes, main_freq_per_bin, &mut prev_main_lo)
        } else {
            (bass_magnitudes, bass_freq_per_bin, &mut prev_bass_lo)
        };

        let f_lo = freq_lo / freq_per_bin;
        let f_hi = freq_hi / freq_per_bin;

        let mut lo = (f_lo as usize).min(magnitudes.len() - 1);
        // Push past the previous bar's bin to avoid reading the same one
        if i > 0 && lo <= *prev_lo && *prev_lo + 1 < magnitudes.len() {
            lo = *prev_lo + 1;
        }
        let hi = (f_hi as usize).max(lo + 1).min(magnitudes.len());
        *prev_lo = lo;

        let sum: f32 = magnitudes[lo..hi].iter().sum();
        let avg = sum / (hi - lo) as f32;

        // Equalizer: boost higher frequencies to compensate for the natural
        // ~1/f roll-off in FFT magnitudes. Without this, highs look dead
        // compared to bass even at the same perceived loudness.
        let eq = (center_freq / min_freq).ln().max(0.0) + 1.0;
        bars[i] = avg * eq;
    }

    bars
}

/// Apply per-band EQ gains to spectrum bars via linear interpolation.
///
/// `eq_gains` defines N gain multipliers evenly distributed across the bar count.
/// Each bar's gain is linearly interpolated between the two nearest EQ bands.
/// Values > 1.0 boost, < 1.0 cut, 1.0 = no change.
pub fn apply_eq(bars: &mut [f32], eq_gains: &[f32]) {
    if eq_gains.is_empty() || bars.is_empty() {
        return;
    }
    if eq_gains.len() == 1 {
        if eq_gains[0] != 1.0 {
            for bar in bars.iter_mut() {
                *bar *= eq_gains[0];
            }
        }
        return;
    }

    let n = bars.len();
    let bands = eq_gains.len();

    for (i, bar) in bars.iter_mut().enumerate() {
        // Map bar index to a position in the EQ band array
        let pos = i as f32 * (bands - 1) as f32 / (n - 1).max(1) as f32;
        let lo = (pos as usize).min(bands - 2);
        let frac = pos - lo as f32;
        let gain = eq_gains[lo] * (1.0 - frac) + eq_gains[lo + 1] * frac;
        *bar *= gain;
    }
}

/// Integral smoothing: additive memory accumulation.
///
/// Each frame: `out = mem * noise_reduction / integral_mod + current`,
/// then `mem = out`. `noise_reduction` is 0.0–1.0. Higher = more memory = smoother.
///
/// Energy accumulates via the integral, and auto-sensitivity normalizes it
/// back by adjusting the gain multiplier.
pub fn smooth(mem: &mut Vec<f32>, bars: &mut [f32], noise_reduction: f32, framerate: f32) {
    if mem.len() != bars.len() {
        *mem = bars.to_vec();
        return;
    }
    let integral_mod = (60.0 / framerate).powf(0.1);

    for (i, bar) in bars.iter_mut().enumerate() {
        *bar += mem[i] * noise_reduction / integral_mod;
        mem[i] = *bar;
    }
}

/// Parabolic bar fall-off. Bars rise instantly to new peaks, then
/// fall along a parabolic curve: `peak * (1 - fall² * gravity_mod)`.
///
/// `fall` increments by a fixed step each frame (scaled for framerate), giving
/// a smooth deceleration from peak to zero.
pub struct Gravity {
    peaks: Vec<f32>,
    falls: Vec<f32>,
    prev: Vec<f32>,
}

/// Per-frame fall increment at 60fps.
const FALL_STEP_60: f32 = 0.028;

impl Gravity {
    pub fn new() -> Self {
        Self {
            peaks: Vec::new(),
            falls: Vec::new(),
            prev: Vec::new(),
        }
    }

    /// Apply parabolic falloff to bars in-place.
    /// `framerate` is the target FPS (used for framerate-independent scaling).
    /// `noise_reduction` controls how quickly bars fall (higher = slower).
    pub fn apply(&mut self, bars: &mut [f32], framerate: f32, noise_reduction: f32) {
        if self.peaks.len() != bars.len() {
            self.peaks = vec![0.0; bars.len()];
            self.falls = vec![0.0; bars.len()];
            self.prev = vec![0.0; bars.len()];
        }

        let framerate_mod = 60.0 / framerate;
        let gravity_mod = framerate_mod.powf(2.5) * 2.0 / noise_reduction.max(0.1);

        for (i, bar) in bars.iter_mut().enumerate() {
            if *bar < self.prev[i] && noise_reduction > 0.1 {
                // Falling: parabolic decay from peak
                *bar = self.peaks[i] * (1.0 - self.falls[i] * self.falls[i] * gravity_mod);
                if *bar < 0.0 {
                    *bar = 0.0;
                }
                self.falls[i] += FALL_STEP_60;
            } else {
                // Rising or equal: snap to new peak
                self.peaks[i] = *bar;
                self.falls[i] = 0.0;
            }
            self.prev[i] = *bar;
        }
    }
}

/// Monstercat smoothing: each bar spreads influence to ALL other
/// bars with exponential distance-based falloff.
///
/// `monstercat` is the falloff parameter (typical: 1.0–2.0). The divisor
/// base is `monstercat * 1.5`, so higher values = steeper falloff = less spread.
/// O(n²) but n is typically < 200 bars so this is fine at audio framerates.
pub fn monstercat(bars: &mut [f32], monstercat: f32) {
    let n = bars.len();
    if n < 2 {
        return;
    }
    let base = monstercat * 1.5;

    for z in 0..n {
        // Spread left
        for m_y in (0..z).rev() {
            let de = (z - m_y) as f32;
            let spread = bars[z] / base.powf(de);
            if spread > bars[m_y] {
                bars[m_y] = spread;
            }
        }
        // Spread right
        for m_y in (z + 1)..n {
            let de = (m_y - z) as f32;
            let spread = bars[z] / base.powf(de);
            if spread > bars[m_y] {
                bars[m_y] = spread;
            }
        }
    }
}

/// Apply a noise floor — zero out any bar below the threshold.
pub fn noise_gate(bars: &mut [f32], floor: f32) {
    for bar in bars.iter_mut() {
        if *bar < floor {
            *bar = 0.0;
        }
    }
}

/// Automatic sensitivity: multiplicative gain adjustment.
///
/// Sensitivity (`sens`) scales all bars. When any bar exceeds 1.0 (overshoot),
/// sens is reduced by 2% per frame. When no overshoot occurs, sens grows by
/// 1% per frame (with a 10% boost during initial ramp-up). All rates are
/// scaled for framerate independence.
///
/// Split into two phases:
///   1. `scale()` — multiply bars by current sens (before gravity/integral)
///   2. `adjust()` — clamp overshoots and update sens (after gravity/integral)
pub struct AutoSensitivity {
    pub sens: f32,
    sens_init: bool,
}

impl AutoSensitivity {
    pub fn new() -> Self {
        Self {
            sens: 1.0,
            sens_init: true,
        }
    }

    /// Phase 1: scale bars by current sensitivity. Call before gravity/integral.
    pub fn scale(&self, bars: &mut [f32]) {
        for bar in bars.iter_mut() {
            *bar *= self.sens;
        }
    }

    /// Phase 2: clamp overshoots and adjust sens for next frame.
    /// Call after gravity and integral smoothing.
    pub fn adjust(&mut self, bars: &mut [f32], framerate: f32, silence: bool) {
        let autosens_mod = (60.0 / framerate).powf(2.0);

        let mut overshoot = false;
        for bar in bars.iter_mut() {
            if *bar > 1.0 {
                overshoot = true;
                *bar = 1.0;
            }
        }

        if overshoot {
            self.sens *= 1.0 - 0.02 * autosens_mod;
            self.sens_init = false;
        } else if !silence {
            self.sens *= 1.0 + 0.01 * autosens_mod;
            if self.sens_init {
                self.sens *= 1.0 + 0.1 * autosens_mod;
            }
        }
    }
}
