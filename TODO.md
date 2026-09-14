# specterm — TODO

## Now
- [ ] Config validation in one place — `low_freq < high_freq <= sample_rate/2` is never checked; clamps for fps/smoothing/bar_width are scattered across `main.rs` #bug

## Next
- [ ] Consider raising default `noise_floor` above 0.0 for quiet-room mic input #improvement
- [ ] Equalizer curve tuning — current `freq_hi^0.85 / log2(FFT_SIZE)` may over/under-boost; needs listening tests #improvement
- [ ] FPS / latency debug overlay (`--debug` flag) #feature
- [ ] Lock-free SPSC ring buffer for audio capture (replace `Arc<Mutex<Vec>>` + `rotate_left`) #improvement

## Later
- [ ] Peak hold indicators on spectrum bars #feature
- [ ] Linux support (PulseAudio/PipeWire monitor sources for system audio capture) #feature
- [ ] Fallback to virtual audio device (BlackHole) if ScreenCaptureKit unavailable #feature

## Scrapped
- Braille rendering — block elements (▁▂▃▄▅▆▇█) give 8 subdivisions per cell vs braille's 4 vertically. Only would help wave/scope modes.

## Done
- [x] Clean tap shutdown on sleep — tap subscribes to `NSWorkspace.willSleepNotification` so replayd sees a clean SCStream disconnect instead of a zombie client across sleep/wake #bug
- [x] Auto-sensitivity cap — `sens` is clamped to `MAX_SENS` in `AutoSensitivity::adjust` #bug
- [x] Fix stereo L/R separation — tap now reads full AudioBufferList so planar SCK audio isn't collapsed to channel 0 only #bug
- [x] Auto-restart wedged capture stream (heals stuck screen-recording indicator after sleep/wake) #bug
- [x] Core audio capture and spectrum rendering with FFT pipeline #feature
- [x] Dual-resolution FFT and logarithmic frequency binning #improvement
- [x] Waveform, oscilloscope, and stereo visualizer modes #feature
- [x] ScreenCaptureKit system audio capture (`specterm-tap`) #feature
- [x] Runtime device switching and enumeration #feature
- [x] TOML-based theme system backed by the shared theme catalog #feature
- [x] Non-blocking settings overlay with live preview #feature
- [x] Auto-sensitivity, monstercat smoothing, gravity fall-off #improvement
- [x] Config persistence for non-theme settings and session-only theme preview #feature
- [x] Now-playing Apple Music track display #feature
