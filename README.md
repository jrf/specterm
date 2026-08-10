# specterm

[![Rust](https://img.shields.io/badge/rust-stable-orange?logo=rust)](https://www.rust-lang.org/)
[![Swift](https://img.shields.io/badge/swift-6.2+-F05138?logo=swift&logoColor=white)](https://swift.org/)
[![macOS](https://img.shields.io/badge/macOS-13%2B-000000?logo=apple&logoColor=white)](https://www.apple.com/macos/)
[![CI](https://github.com/jrf/specterm/actions/workflows/ci.yml/badge.svg)](https://github.com/jrf/specterm/actions/workflows/ci.yml)

Terminal audio visualizer for macOS. Renders real-time spectrum bars, waveforms, oscilloscopes, and stereo visualizations from mic input or system audio.

| | |
|---|---|
| ![spectrum](spectrum.png) | ![waveform](waveform.png) |
| ![oscilloscope](oscilloscope.png) | ![stereo](stereo.png) |

## Install

Requires Rust and (optionally) Swift for system audio capture.

```fish
just install
```

This builds and installs `specterm` and `specterm-tap` to `~/.cargo/bin`.

Maintainers can use `just package <version>` to build an ad-hoc-signed DMG for
local packaging validation. Public releases are Developer ID-signed and
notarized by the tag-driven GitHub Actions workflow; see `RELEASING.md`.

## Usage

```
specterm                        # spectrum visualizer (system audio by default)
specterm --mode wave            # waveform mode
specterm --mode scope           # oscilloscope mode
specterm --mode stereo          # stereo L/R visualization
specterm --device system        # capture system audio (requires specterm-tap)
specterm --device "MacBook Pro Microphone"
specterm --theme ~/.config/themes/fire.toml
specterm --bar-width 1 --bar-spacing 0
specterm --low-freq 40 --high-freq 16000
specterm --list-devices         # list available audio devices
```

Run `specterm --help` for all command-line options. Bar count adapts to the
terminal width and the configured bar width and spacing.

## Keybindings

| Key | Action |
|-----|--------|
| `?` | Help |
| `m` | Cycle visualization mode |
| `d` | Select audio device |
| `s` | Settings (theme, smoothing, noise, etc.) |
| `Up` / `Down` | Increase / decrease sensitivity |
| `Right` / `Left` | More / fewer bars |
| `q` / `Esc` | Quit |

## Modes

- **spectrum** — frequency bars with color gradient and gravity fall-off
- **wave** — real-time waveform amplitude plot
- **scope** — oscilloscope with zero-crossing trigger
- **stereo** — left channel bars up, right channel bars down from center

## Themes

Specterm reads explicit `theme` and `theme_catalog` paths from
`~/.config/specterm/config.toml`. The selected file is loaded directly; the
catalog's `themes = [...]` array supplies the settings picker. Specterm never
scans a theme directory. Palettes without a `[visualizer]` section receive a
gradient from their semantic color range automatically.

```toml
# ~/.config/specterm/config.toml
theme = "~/.config/themes/synthwave.toml"
theme_catalog = "~/.config/themes/catalog.toml"
```

```toml
# ~/.config/themes/catalog.toml
themes = [
  "~/.config/themes/synthwave.toml",
  "~/.config/themes/tokyo-night-moon.toml",
]
```

A theme defines named colors and may provide Specterm-specific roles:

```toml
[colors]
deep_purple = "#2b0057"
hot_pink = "#ff2975"
cyan = "#00e5ff"

[visualizer]
gradient = ["deep_purple", "hot_pink", "cyan"]
wave_color = "hot_pink"
scope_color = "cyan"
```

## Settings

Press `s` to open the settings overlay (visualizer keeps running behind it). Adjust with arrow keys or vim bindings:

- **Theme** — cycle through color themes with live preview
- **Smoothing** — temporal smoothing between frames (0.0–0.99)
- **Monstercat** — smooth envelope connecting bar tops
- **Noise floor** — threshold to zero out quiet bars
- **Gradient mode** — color by amplitude or by bar position
- **Bar width** — width of each bar in columns (1–8)
- **Bar spacing** — gap between bars (0–4)
- **Sensitivity** — manual gain adjustment (10–500%)
- **EQ bands** — per-band gain from 0.0–3.0

Theme changes in the settings picker always apply to the current session only and never rewrite `config.toml`. The `--save` flag and `save_settings = true` can persist other in-app settings, but they never replace the configured `theme`; edit that path directly to change the startup theme.

## System audio capture

To visualize audio from Apple Music or other apps, specterm uses a companion Swift binary (`specterm-tap`) that captures system audio via ScreenCaptureKit.

**Requirements:**
- macOS 13+
- Screen Recording permission must be granted to your terminal app (System Settings > Privacy & Security > Screen Recording)

Select "System Audio (ScreenCaptureKit)" from the device menu (`d`), or pass `--device system`.

Specterm shuts down its ScreenCaptureKit stream and exits before macOS sleeps,
including clamshell sleep, so the capture service is not carried across a
sleep/wake cycle. Closing the lid in external-display clamshell mode does not
exit Specterm when macOS remains awake.

**Alternative:** Install [BlackHole](https://github.com/ExistentialAudio/BlackHole), create a Multi-Output Device in Audio MIDI Setup (speakers + BlackHole), set it as your output, then select "BlackHole 2ch" as the input device.

## Development

```fish
just build       # Rust and Swift debug builds
just fmt-check   # Rust formatting
just lint        # Rust Clippy with warnings denied
just test        # Rust unit tests
swift test --package-path tap
```

GitHub Actions runs the Rust formatting, lint, and test checks plus the Swift
test suite on pushes and pull requests.
