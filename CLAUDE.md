# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What is sonitus

Terminal audio visualizer for macOS. Renders real-time spectrum bars, waveforms, and oscilloscopes from mic input or system audio. Built with Rust (cpal + ratatui) and a companion Swift binary (`sonitus-tap`) for system audio capture via ScreenCaptureKit.

## Build & Development Commands

Uses `just` as the task runner:

- `just build` — debug build (Rust + Swift)
- `just release` — release build
- `just install` — build release and install both binaries to `~/.cargo/bin`
- `just run [ARGS]` — debug run (e.g. `just run --mode wave --theme fire`)
- `just lint` — `cargo clippy -- -D warnings`
- `just fmt` — `cargo fmt`
- `just test` — `cargo test`
- `just clean` — clean both Rust and Swift build artifacts

The Swift tap binary must be built separately (`cd tap && swift build`) or via `just build`/`just install`.

## Architecture

Two-process design: the Rust binary handles all visualization, the Swift binary (`tap/`) only captures system audio.

**Main loop** (`main.rs`): CLI parsing (clap) → load config → start audio capture → render loop at target FPS. The render loop is: read samples from shared buffer → FFT → bin/smooth → draw.

**Data flow**: Audio thread writes interleaved samples into `Arc<Mutex<Vec<f32>>>` ring buffers (mono + stereo pair). Render thread reads from these buffers each frame.

**Key modules**:
- `audio.rs` — Two capture paths: `start_capture` (cpal device input) and `start_tap` (spawns `sonitus-tap` subprocess, reads raw f32 from its stdout). `CaptureHandle` enum keeps the stream/child alive via RAII.
- `analysis.rs` — Hann-windowed FFT (`FFT_SIZE = 2048`), logarithmic frequency binning, exponential frame smoothing, monstercat envelope smoothing, noise gate, and `AutoSensitivity` (auto-gain normalization).
- `render.rs` — All ratatui terminal UI: spectrum (BarChart), waveform/oscilloscope (Canvas with line segments), stereo (Canvas with mirrored bars), plus interactive menus (device, theme, settings, help).
- `theme.rs` — Static `THEMES` array of gradient color definitions.
- `config.rs` — Persists settings to `~/.config/sonitus/config.toml` via serde/toml. Settings changed at runtime (theme, smoothing, etc.) are saved automatically.

**System audio** (`tap/Sources/main.swift`): Uses ScreenCaptureKit to capture system audio output, mixes to mono f32, writes raw bytes to stdout. Requires macOS 13+ and Screen Recording permission.
