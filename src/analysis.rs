//! DSP: FFT spectrum analysis, waveform extraction, and smoothing.

use rustfft::{num_complex::Complex, FftPlanner};

/// FFT window size. Must be a power of two.
pub const FFT_SIZE: usize = 2048;

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

    // TODO: apply a window function (Hann) before FFT
    fft.process(&mut buffer);

    // Positive frequencies only
    buffer[..FFT_SIZE / 2]
        .iter()
        .map(|c| c.norm() / FFT_SIZE as f32)
        .collect()
}

/// Bin a full spectrum into `n` bars for display, using logarithmic frequency scaling.
pub fn bin_spectrum(magnitudes: &[f32], n: usize) -> Vec<f32> {
    // TODO: logarithmic binning so low frequencies get more bars
    // For now, linear binning as placeholder
    let bin_size = magnitudes.len() / n;
    (0..n)
        .map(|i| {
            let start = i * bin_size;
            let end = (start + bin_size).min(magnitudes.len());
            magnitudes[start..end].iter().sum::<f32>() / bin_size as f32
        })
        .collect()
}

/// Smooth between consecutive frames to reduce flickering.
pub fn smooth(prev: &[f32], current: &[f32], factor: f32) -> Vec<f32> {
    prev.iter()
        .zip(current.iter())
        .map(|(p, c)| p * factor + c * (1.0 - factor))
        .collect()
}
