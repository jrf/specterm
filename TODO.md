# termwave — TODO

## Now

## Next
- [ ] Auto-sensitivity cap — `sens` grows unbounded for quiet audio (1% per frame compounds to 392x in 10s); add a max cap and possibly raise default `noise_floor` above 0.0 #bug
- [ ] Equalizer curve tuning — current `ln(freq/low_freq)+1.0` may over/under-boost; needs listening tests #improvement
- [ ] FPS / latency debug overlay (`--debug` flag) #feature

## Later
- [ ] Peak hold indicators on spectrum bars #feature
- [ ] Linux support (PulseAudio/PipeWire monitor sources for system audio capture) #feature
- [ ] Fallback to virtual audio device (BlackHole) if ScreenCaptureKit unavailable #feature

## Scrapped
- Braille rendering — block elements (▁▂▃▄▅▆▇█) give 8 subdivisions per cell vs braille's 4 vertically. Only would help wave/scope modes.

## Done
- [x] Core audio capture and spectrum rendering with FFT pipeline #feature
- [x] Dual-resolution FFT and logarithmic frequency binning #improvement
- [x] Waveform, oscilloscope, and stereo visualizer modes #feature
- [x] ScreenCaptureKit system audio capture (`termwave-tap`) #feature
- [x] Runtime device switching and enumeration #feature
- [x] TOML-based theme system with 8 built-in themes #feature
- [x] Non-blocking settings overlay with live preview #feature
- [x] Auto-sensitivity, monstercat smoothing, gravity fall-off #improvement
- [x] Config persistence and theme-aware UI #feature
- [x] Now-playing Apple Music track display #feature
