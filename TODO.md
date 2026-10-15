# specterm — TODO

## Now
- [ ] Auto-sensitivity attack — loud jumps take ~1.8 s (+20 dB) to ~3.7 s (+40 dB) to recover their shape because sens can only drop 2%/frame. Faster cuts plus rescaling gravity/smoothing state fix that but deepen dips after short bursts (−6 dB → −15 dB at 7%/frame); needs a hold or faster release redesign #improvement
- [ ] Bass resolution — single 4096-point FFT (11.7 Hz bins) leaves 11 of 60 default bars narrower than one bin, shifting 100–200 Hz content to the wrong bars. A dual-resolution FFT existed and was removed in 9baa0f0 to simplify the pipeline; a re-add needs a latency budget (8192 = ~170 ms) or a constant-Q transform #improvement

## Next
- [ ] Consider raising default `noise_floor` above 0.0 for quiet-room mic input #improvement
- [ ] Equalizer curve tuning — `freq_hi^0.85` shows pink noise rising +2.1 dB/oct but a typical music spectrum (−4.5 dB/oct) nearly flat (+0.6 dB/oct). `0.5` makes pink noise flat (−0.0) but music falls (−1.5 dB/oct); `0.7` makes music flat (−0.3) with pink at +1.2. Pick by listening #improvement
- [ ] FPS / latency debug overlay (`--debug` flag) #feature
- [ ] Lock-free SPSC ring buffer for audio capture (replace `Arc<Mutex<Vec>>` + `rotate_left`) #improvement

## Later
- [ ] Peak hold indicators on spectrum bars #feature
- [ ] Linux support (PulseAudio/PipeWire monitor sources for system audio capture) #feature
- [ ] Fallback to virtual audio device (BlackHole) if ScreenCaptureKit unavailable #feature

## Scrapped
- Braille rendering — block elements (▁▂▃▄▅▆▇█) give 8 subdivisions per cell vs braille's 4 vertically. Only would help wave/scope modes.

## Done
- [x] Config validation in one place — `Config::validated` clamps every field, rejects unusable frequency ranges, caps `high_freq` at Nyquist, and reports corrections #bug
- [x] Stereo shares one auto-sensitivity gain across L/R so balance and panning stay visible #bug
- [x] Smoothing is an EMA, so brightness no longer depends on framerate #bug
- [x] Waveform draws per-column min/max (no aliasing); scope shows a fixed-length triggered window (constant timebase) #bug
- [x] Clean tap shutdown on sleep — tap subscribes to `NSWorkspace.willSleepNotification` so replayd sees a clean SCStream disconnect instead of a zombie client across sleep/wake #bug
- [x] Auto-sensitivity cap — `sens` is clamped to `MAX_SENS` in `AutoSensitivity::adjust` #bug
- [x] Fix stereo L/R separation — tap now reads full AudioBufferList so planar SCK audio isn't collapsed to channel 0 only #bug
- [x] Auto-restart wedged capture stream (heals stuck screen-recording indicator after sleep/wake) #bug
- [x] Core audio capture and spectrum rendering with FFT pipeline #feature
- [x] Logarithmic frequency binning (dual-resolution FFT was added, then removed in 9baa0f0) #improvement
- [x] Waveform, oscilloscope, and stereo visualizer modes #feature
- [x] ScreenCaptureKit system audio capture (`specterm-tap`) #feature
- [x] Runtime device switching and enumeration #feature
- [x] TOML-based theme system backed by the shared theme catalog #feature
- [x] Non-blocking settings overlay with live preview #feature
- [x] Auto-sensitivity, monstercat smoothing, gravity fall-off #improvement
- [x] Config persistence for non-theme settings and session-only theme preview #feature
- [x] Now-playing Apple Music track display #feature
