//! Audio capture from system input devices via cpal.
//!
//! Captures audio samples from the default input device and writes them
//! into a shared ring buffer for the analysis/render pipeline to consume.

use std::sync::{Arc, Mutex};

use anyhow::Result;

/// Shared sample buffer between audio capture thread and render thread.
pub type SampleBuffer = Arc<Mutex<Vec<f32>>>;

/// Create a new shared sample buffer.
pub fn new_buffer(capacity: usize) -> SampleBuffer {
    Arc::new(Mutex::new(vec![0.0; capacity]))
}

/// Start capturing audio into the shared buffer.
///
/// Returns the sample rate of the captured stream.
pub fn start_capture(_buffer: SampleBuffer, _device: Option<&str>) -> Result<u32> {
    // TODO:
    // 1. Enumerate devices or pick default input
    // 2. Open stream with cpal
    // 3. In the data callback, write samples into the shared buffer
    // 4. Return the stream's sample rate
    todo!("audio capture not yet implemented")
}
