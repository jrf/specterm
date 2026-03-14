//! DSP: FFT spectrum analysis, waveform extraction, and smoothing.

use std::f32::consts::PI;

use rustfft::{num_complex::Complex, FftPlanner};

/// FFT window size. Must be a power of two.
pub const FFT_SIZE: usize = 2048;

/// Apply a Hann window to samples in-place.
fn hann_window(samples: &mut [Complex<f32>]) {
    let n = samples.len() as f32;
    for (i, s) in samples.iter_mut().enumerate() {
        let w = 0.5 * (1.0 - (2.0 * PI * i as f32 / n).cos());
        s.re *= w;
    }
}

/// Compute the magnitude spectrum from a buffer of time-domain samples.
///
/// Returns `FFT_SIZE / 2` magnitude values (positive frequencies only).
pub fn spectrum(samples: &[f32]) -> Vec<f32> {
    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(FFT_SIZE);

    let mut buffer: Vec<Complex<f32>> = samples
        .iter()
        .take(FFT_SIZE)
        .map(|&s| Complex { re: s, im: 0.0 })
        .collect();

    // Zero-pad if we have fewer samples than FFT_SIZE
    buffer.resize(FFT_SIZE, Complex { re: 0.0, im: 0.0 });

    hann_window(&mut buffer);
    fft.process(&mut buffer);

    // Positive frequencies only, convert to magnitude
    buffer[..FFT_SIZE / 2]
        .iter()
        .map(|c| c.norm() / FFT_SIZE as f32)
        .collect()
}

/// Bin a full spectrum into `n` bars using logarithmic frequency scaling.
///
/// `low_freq` and `high_freq` control the visible frequency range.
pub fn bin_spectrum(
    magnitudes: &[f32],
    n: usize,
    sample_rate: u32,
    low_freq: f32,
    high_freq: f32,
) -> Vec<f32> {
    if magnitudes.is_empty() || n == 0 {
        return vec![0.0; n];
    }

    let nyquist = sample_rate as f32 / 2.0;
    let freq_per_bin = nyquist / magnitudes.len() as f32;

    let min_freq = low_freq.max(1.0);
    let max_freq = high_freq.min(nyquist);
    let log_min = min_freq.ln();
    let log_max = max_freq.ln();

    let mut bars = vec![0.0f32; n];

    for i in 0..n {
        let f_lo = ((log_min + (log_max - log_min) * i as f32 / n as f32).exp()) / freq_per_bin;
        let f_hi =
            ((log_min + (log_max - log_min) * (i + 1) as f32 / n as f32).exp()) / freq_per_bin;

        let lo = (f_lo as usize).min(magnitudes.len() - 1);
        let hi = (f_hi as usize).max(lo + 1).min(magnitudes.len());

        let sum: f32 = magnitudes[lo..hi].iter().sum();
        bars[i] = sum / (hi - lo) as f32;
    }

    bars
}

/// Smooth between consecutive frames to reduce flickering.
///
/// `factor` controls decay: 0.0 = no smoothing, 0.9 = heavy smoothing.
pub fn smooth(prev: &[f32], current: &[f32], factor: f32) -> Vec<f32> {
    if prev.len() != current.len() {
        return current.to_vec();
    }
    prev.iter()
        .zip(current.iter())
        .map(|(p, c)| p * factor + c * (1.0 - factor))
        .collect()
}

/// Monstercat-style smoothing: each peak spreads influence to its neighbors
/// with exponential falloff, creating a smooth connected envelope.
///
/// `strength` controls the falloff rate (0.5–0.9 typical, higher = wider spread).
pub fn monstercat(bars: &mut [f32], strength: f32) {
    let n = bars.len();
    if n < 2 {
        return;
    }

    // Forward pass: each bar pulls up its right neighbor
    for i in 1..n {
        let prev = bars[i - 1] * strength;
        if prev > bars[i] {
            bars[i] = prev;
        }
    }

    // Backward pass: each bar pulls up its left neighbor
    for i in (0..n - 1).rev() {
        let next = bars[i + 1] * strength;
        if next > bars[i] {
            bars[i] = next;
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

/// Automatic sensitivity: tracks a rolling peak and normalizes bars
/// so the display uses the full height regardless of volume.
pub struct AutoSensitivity {
    peak: f32,
    /// How fast the peak decays toward the current max (per frame).
    /// Lower = slower decay = more stable. 0.01–0.05 typical.
    decay: f32,
    /// Minimum peak to prevent division by tiny numbers during silence.
    min_peak: f32,
}

impl AutoSensitivity {
    pub fn new() -> Self {
        Self {
            peak: 0.001,
            decay: 0.05,
            min_peak: 0.0001,
        }
    }

    /// Normalize bars in-place based on tracked peak.
    /// Returns the current sensitivity peak for diagnostics.
    pub fn apply(&mut self, bars: &mut [f32]) -> f32 {
        let current_max = bars.iter().cloned().fold(0.0f32, f32::max);

        // If current frame is louder, jump up immediately
        if current_max > self.peak {
            self.peak = current_max;
        } else {
            // Slowly decay toward current level
            self.peak = self.peak * (1.0 - self.decay) + current_max * self.decay;
        }

        let peak = self.peak.max(self.min_peak);

        for bar in bars.iter_mut() {
            *bar /= peak;
        }

        peak
    }
}
