//! Audio capture from system input devices via cpal, or from specterm-tap subprocess.

use std::io::Read as _;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Instant;

use anyhow::{Context, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::Stream;

/// Shared sample buffer between audio capture thread and render thread.
pub type SampleBuffer = Arc<Mutex<Vec<f32>>>;

/// Stereo sample buffers (left, right).
pub type StereoPair = (SampleBuffer, SampleBuffer);

/// Shared now-playing track info from the tap subprocess.
pub type NowPlaying = Arc<Mutex<Option<String>>>;

pub fn new_now_playing() -> NowPlaying {
    Arc::new(Mutex::new(None))
}

/// Tracks when samples were last written. If no new data arrives for a while,
/// the render thread can treat the buffer as silence instead of showing stale data.
pub struct LastWriteTime(Arc<Mutex<Instant>>);

impl LastWriteTime {
    pub fn new() -> Self {
        Self(Arc::new(Mutex::new(Instant::now())))
    }

    pub fn clone_inner(&self) -> Arc<Mutex<Instant>> {
        self.0.clone()
    }

    /// How long since the audio thread last wrote samples.
    pub fn elapsed(&self) -> std::time::Duration {
        self.0.lock().unwrap().elapsed()
    }

    /// Reset the timestamp to now. Used after restarting a stalled capture so
    /// the stall detector doesn't immediately re-trigger before new samples arrive.
    pub fn mark(&self) {
        *self.0.lock().unwrap() = Instant::now();
    }
}

/// Create a new shared sample buffer.
pub fn new_buffer(capacity: usize) -> SampleBuffer {
    Arc::new(Mutex::new(vec![0.0; capacity]))
}

/// Create a stereo pair of buffers.
pub fn new_stereo_buffers(capacity: usize) -> StereoPair {
    (new_buffer(capacity), new_buffer(capacity))
}

/// Handle for an active audio capture — either a cpal stream or a tap subprocess.
/// The Stream/Child are held to keep them alive; they're used via Drop, not read.
#[allow(dead_code)]
pub enum CaptureHandle {
    Device(Stream),
    Tap(Child),
}

impl CaptureHandle {
    /// Returns true if this is a tap capture whose subprocess has exited.
    pub fn tap_exited(&mut self) -> bool {
        if let CaptureHandle::Tap(ref mut child) = self {
            matches!(child.try_wait(), Ok(Some(_)))
        } else {
            false
        }
    }
}

impl Drop for CaptureHandle {
    fn drop(&mut self) {
        match self {
            CaptureHandle::Device(ref stream) => {
                // Explicitly pause the cpal stream before dropping to ensure
                // CoreAudio fully releases the device.
                let _ = stream.pause();
            }
            CaptureHandle::Tap(ref mut child) => {
                shutdown_tap(child);
            }
        }
    }
}

/// Send SIGTERM to the tap and wait briefly for it to exit cleanly so
/// ScreenCaptureKit can stop the stream (clears the macOS screen-recording
/// indicator). Falls back to SIGKILL if it doesn't exit in time.
#[cfg_attr(test, allow(dead_code))]
fn shutdown_tap(child: &mut Child) {
    #[cfg(unix)]
    unsafe {
        libc::kill(child.id() as libc::pid_t, libc::SIGTERM);
    }

    let deadline = Instant::now() + std::time::Duration::from_millis(500);
    while Instant::now() < deadline {
        match child.try_wait() {
            Ok(Some(_)) => return,
            Ok(None) => thread::sleep(std::time::Duration::from_millis(20)),
            Err(_) => break,
        }
    }

    let _ = child.kill();
    let _ = child.wait();
}

/// Start capturing audio into the shared buffers from a device.
///
/// Writes mono-mixed data to `mono_buf`, and separate L/R to `stereo`.
pub fn start_capture(
    mono_buf: SampleBuffer,
    stereo: StereoPair,
    device_name: Option<&str>,
    last_write: &LastWriteTime,
) -> Result<(u32, CaptureHandle)> {
    let host = cpal::default_host();

    let device = match device_name {
        Some(name) => host
            .input_devices()?
            .find(|d| d.name().map(|n| n == name).unwrap_or(false))
            .context(format!("audio device '{}' not found", name))?,
        None => host
            .default_input_device()
            .context("no default input device available")?,
    };

    clear_buffers(&mono_buf, &stereo);

    let config = device.default_input_config()?;
    let sample_rate = config.sample_rate().0;
    let channels = config.channels() as usize;

    let err_fn = |err: cpal::StreamError| {
        eprintln!("audio stream error: {}", err);
    };

    let lw = last_write.clone_inner();

    let stream = match config.sample_format() {
        cpal::SampleFormat::F32 => {
            let mb = mono_buf.clone();
            let st = (stereo.0.clone(), stereo.1.clone());
            let lw = lw.clone();
            let mut scratch = Scratch::new();
            device.build_input_stream(
                &config.into(),
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    scratch.fill(data.iter().copied(), channels);
                    scratch.flush(&mb, &st, &lw);
                },
                err_fn,
                None,
            )?
        }
        cpal::SampleFormat::I16 => {
            let mb = mono_buf.clone();
            let st = (stereo.0.clone(), stereo.1.clone());
            let lw = lw.clone();
            let mut scratch = Scratch::new();
            device.build_input_stream(
                &config.into(),
                move |data: &[i16], _: &cpal::InputCallbackInfo| {
                    scratch.fill(data.iter().map(|&s| s as f32 / 32768.0), channels);
                    scratch.flush(&mb, &st, &lw);
                },
                err_fn,
                None,
            )?
        }
        cpal::SampleFormat::U16 => {
            let mb = mono_buf.clone();
            let st = (stereo.0.clone(), stereo.1.clone());
            let lw = lw.clone();
            let mut scratch = Scratch::new();
            device.build_input_stream(
                &config.into(),
                move |data: &[u16], _: &cpal::InputCallbackInfo| {
                    scratch.fill(
                        data.iter()
                            .map(|&s| (s as f32 / u16::MAX as f32) * 2.0 - 1.0),
                        channels,
                    );
                    scratch.flush(&mb, &st, &lw);
                },
                err_fn,
                None,
            )?
        }
        _ => anyhow::bail!("unsupported sample format"),
    };

    stream.play()?;
    Ok((sample_rate, CaptureHandle::Device(stream)))
}

/// Start capturing system audio via specterm-tap subprocess.
pub fn start_tap(
    mono_buf: SampleBuffer,
    stereo: StereoPair,
    sample_rate: u32,
    last_write: &LastWriteTime,
) -> Result<(u32, CaptureHandle)> {
    let tap_bin = find_tap_binary();

    clear_buffers(&mono_buf, &stereo);

    let mut child = Command::new(&tap_bin)
        .args(["--sample-rate", &sample_rate.to_string()])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .context(format!(
            "failed to start specterm-tap (looked for '{}'). \
             Build it with: cd tap && swift build -c release",
            tap_bin
        ))?;

    let mut stdout = child.stdout.take().context("failed to get tap stdout")?;

    // The tap outputs interleaved stereo (L, R, L, R...).
    // Split into L/R channels and mix to mono.
    let lw = last_write.clone_inner();
    thread::spawn(move || {
        let mut read_buf = [0u8; 4096];
        let mut scratch = Scratch::new();
        loop {
            match stdout.read(&mut read_buf) {
                Ok(0) => break,
                Ok(n) => {
                    // Align to 8 bytes (one stereo frame = 2 × f32 = 8 bytes)
                    let sample_bytes = &read_buf[..n - (n % 8)];
                    if sample_bytes.is_empty() {
                        continue;
                    }
                    scratch.fill(
                        sample_bytes
                            .chunks_exact(4)
                            .map(|chunk| f32::from_ne_bytes(chunk.try_into().unwrap())),
                        2,
                    );
                    scratch.flush(&mono_buf, &stereo, &lw);
                }
                Err(_) => break,
            }
        }
    });

    Ok((sample_rate, CaptureHandle::Tap(child)))
}

fn clear_buffers(mono: &SampleBuffer, stereo: &StereoPair) {
    mono.lock().unwrap().fill(0.0);
    stereo.0.lock().unwrap().fill(0.0);
    stereo.1.lock().unwrap().fill(0.0);
}

/// Find the specterm-tap binary: check next to our own executable first, then PATH.
fn find_tap_binary() -> String {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let candidate = dir.join("specterm-tap");
            if candidate.exists() {
                return candidate.to_string_lossy().into_owned();
            }
        }
    }
    "specterm-tap".to_string()
}

/// Append `samples` to the tail of a fixed-capacity ring buffer, dropping the
/// oldest samples. Does not touch the write timestamp.
fn write_to_buffer(buffer: &SampleBuffer, samples: &[f32]) {
    if samples.is_empty() {
        return;
    }
    let mut buf = buffer.lock().unwrap();
    let capacity = buf.len();
    let new_len = samples.len();

    if new_len >= capacity {
        buf.copy_from_slice(&samples[new_len - capacity..]);
    } else {
        buf.rotate_left(new_len);
        buf[capacity - new_len..].copy_from_slice(samples);
    }
}

/// Reusable de-interleave scratch space owned by a capture thread.
///
/// Audio callbacks run on a realtime thread, so we avoid allocating there:
/// the vectors grow to the callback size once and are reused afterwards.
struct Scratch {
    mono: Vec<f32>,
    left: Vec<f32>,
    right: Vec<f32>,
}

impl Scratch {
    fn new() -> Self {
        Self {
            mono: Vec::new(),
            left: Vec::new(),
            right: Vec::new(),
        }
    }

    /// De-interleave `samples` (with `channels` samples per frame) into
    /// mono-mix, left, and right. Channels beyond the second are ignored;
    /// mono input is duplicated to both sides. A trailing partial frame is dropped.
    fn fill<I: Iterator<Item = f32>>(&mut self, samples: I, channels: usize) {
        self.mono.clear();
        self.left.clear();
        self.right.clear();
        let channels = channels.max(1);

        let mut idx = 0;
        let mut l = 0.0;
        let mut r = 0.0;
        for s in samples {
            match idx {
                0 => l = s,
                1 => r = s,
                _ => {}
            }
            idx += 1;
            if idx == channels {
                if channels == 1 {
                    r = l;
                }
                self.mono.push((l + r) / 2.0);
                self.left.push(l);
                self.right.push(r);
                idx = 0;
            }
        }
    }

    /// Push the de-interleaved samples into the shared ring buffers and stamp
    /// the write time once.
    fn flush(&self, mono_buf: &SampleBuffer, stereo: &StereoPair, last_write: &Mutex<Instant>) {
        if self.mono.is_empty() {
            return;
        }
        write_to_buffer(mono_buf, &self.mono);
        write_to_buffer(&stereo.0, &self.left);
        write_to_buffer(&stereo.1, &self.right);
        *last_write.lock().unwrap() = Instant::now();
    }
}

/// List available input device names.
pub fn list_devices() -> Result<Vec<String>> {
    let host = cpal::default_host();
    let mut devices: Vec<String> = host
        .input_devices()?
        .filter_map(|d| d.name().ok())
        .collect();

    let tap_bin = find_tap_binary();
    if std::path::Path::new(&tap_bin).exists() || which_exists(&tap_bin) {
        devices.insert(0, SYSTEM_AUDIO_LABEL.to_string());
    }

    Ok(devices)
}

/// Check if a binary name exists on PATH.
fn which_exists(name: &str) -> bool {
    Command::new("which")
        .arg(name)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Label used in the device menu for system audio capture.
pub const SYSTEM_AUDIO_LABEL: &str = "System Audio (ScreenCaptureKit)";

const NOW_PLAYING_SCRIPT: &str = r#"
tell application "System Events"
    if not (exists process "Music") then return ""
end tell
tell application "Music"
    if player state is playing then
        set a to artist of current track
        set n to name of current track
        return a & " — " & n
    end if
end tell
return ""
"#;

/// Poll now-playing info from Music.app via osascript in a background thread.
/// Updates `now_playing` every `interval`. Runs until the process exits.
pub fn start_now_playing_poller(now_playing: NowPlaying, interval: std::time::Duration) {
    thread::spawn(move || {
        // Small initial delay before first poll
        thread::sleep(std::time::Duration::from_secs(1));
        let mut last_track = String::new();
        loop {
            let track = query_now_playing().unwrap_or_default();
            if track != last_track {
                last_track = track.clone();
                *now_playing.lock().unwrap() = if track.is_empty() { None } else { Some(track) };
            }
            thread::sleep(interval);
        }
    });
}

fn query_now_playing() -> Option<String> {
    let output = Command::new("/usr/bin/osascript")
        .args(["-e", NOW_PLAYING_SCRIPT])
        .stderr(Stdio::null())
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if text.is_empty() {
        None
    } else {
        Some(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn contents(buf: &SampleBuffer) -> Vec<f32> {
        buf.lock().unwrap().clone()
    }

    #[test]
    fn ring_buffer_appends_and_drops_oldest() {
        let buf = new_buffer(4);
        write_to_buffer(&buf, &[1.0, 2.0]);
        assert_eq!(contents(&buf), vec![0.0, 0.0, 1.0, 2.0]);
        write_to_buffer(&buf, &[3.0, 4.0, 5.0]);
        assert_eq!(contents(&buf), vec![2.0, 3.0, 4.0, 5.0]);
    }

    #[test]
    fn ring_buffer_keeps_tail_when_input_exceeds_capacity() {
        let buf = new_buffer(3);
        write_to_buffer(&buf, &[1.0, 2.0, 3.0, 4.0, 5.0]);
        assert_eq!(contents(&buf), vec![3.0, 4.0, 5.0]);
        write_to_buffer(&buf, &[]);
        assert_eq!(contents(&buf), vec![3.0, 4.0, 5.0]);
    }

    #[test]
    fn scratch_deinterleaves_stereo() {
        let mut s = Scratch::new();
        s.fill([1.0, -1.0, 0.5, 0.5].into_iter(), 2);
        assert_eq!(s.left, vec![1.0, 0.5]);
        assert_eq!(s.right, vec![-1.0, 0.5]);
        assert_eq!(s.mono, vec![0.0, 0.5]);
    }

    #[test]
    fn scratch_duplicates_mono_and_ignores_extra_channels() {
        let mut s = Scratch::new();
        s.fill([0.2, 0.4].into_iter(), 1);
        assert_eq!(s.left, vec![0.2, 0.4]);
        assert_eq!(s.right, vec![0.2, 0.4]);
        assert_eq!(s.mono, vec![0.2, 0.4]);

        // 4-channel input: only first two channels used; trailing partial frame dropped.
        s.fill([1.0, 2.0, 9.0, 9.0, 3.0, 4.0, 9.0, 9.0, 5.0].into_iter(), 4);
        assert_eq!(s.left, vec![1.0, 3.0]);
        assert_eq!(s.right, vec![2.0, 4.0]);
    }

    #[test]
    fn scratch_flush_writes_all_buffers_and_stamps_time() {
        let mono = new_buffer(2);
        let stereo = new_stereo_buffers(2);
        let lw = Mutex::new(Instant::now() - std::time::Duration::from_secs(10));
        let mut s = Scratch::new();
        s.fill([1.0, 3.0].into_iter(), 2);
        s.flush(&mono, &stereo, &lw);
        assert_eq!(contents(&mono), vec![0.0, 2.0]);
        assert_eq!(contents(&stereo.0), vec![0.0, 1.0]);
        assert_eq!(contents(&stereo.1), vec![0.0, 3.0]);
        assert!(lw.lock().unwrap().elapsed().as_secs() < 5);
    }
}
