# termwave — TODO

## Now
- [ ] Peak hold indicators on spectrum bars
- [ ] Braille character rendering for sub-cell resolution

## Next
- [ ] Stereo: side-by-side or overlaid L/R channels
- [ ] Frequency axis labels (Hz)
- [ ] FPS / latency debug overlay (`--debug` flag)

## Later
- [ ] Fallback to virtual audio device (BlackHole) if ScreenCaptureKit unavailable

## Done
- [x] Audio capture: open default input device with cpal, write samples to shared ring buffer
- [x] Spectrum rendering: draw frequency bars using ratatui BarChart
- [x] Wire main loop: audio thread -> ring buffer -> FFT -> render at ~30fps
- [x] Handle terminal resize gracefully
- [x] Quit on `q`, Esc, or Ctrl+C with clean terminal restore
- [x] Apply Hann window before FFT
- [x] Logarithmic frequency binning (more bars for bass, fewer for treble)
- [x] Frame smoothing (exponential decay between frames)
- [x] Color gradient on spectrum bars
- [x] Waveform mode: plot amplitude across terminal width
- [x] Oscilloscope mode: zero-crossing triggered waveform display
- [x] ScreenCaptureKit integration (macOS 13+) for capturing system audio output
- [x] Device enumeration and selection via `--device` flag / `--list-devices`
- [x] Runtime device switching via `d` keybinding
- [x] Configurable color themes (8 built-in: classic, fire, ocean, purple, matrix, synthwave, tokyo-night-moon, mono)
