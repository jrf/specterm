mod analysis;
mod audio;
mod config;
mod render;
mod theme;

use std::time::{Duration, Instant};

use anyhow::Result;
use clap::Parser;

#[derive(Parser)]
#[command(name = "sonitus", about = "Terminal audio visualizer")]
struct Cli {
    /// Visualization mode
    #[arg(short, long)]
    mode: Option<String>,

    /// Audio input device (defaults to system default, use "system" for system audio)
    #[arg(short, long)]
    device: Option<String>,

    /// Color theme (classic, fire, ocean, purple, matrix, mono)
    #[arg(short, long)]
    theme: Option<String>,

    /// Target frames per second
    #[arg(long)]
    fps: Option<u64>,

    /// Number of spectrum bars
    #[arg(short, long)]
    bars: Option<usize>,

    /// Low frequency cutoff in Hz
    #[arg(long)]
    low_freq: Option<f32>,

    /// High frequency cutoff in Hz
    #[arg(long)]
    high_freq: Option<f32>,

    /// Noise floor threshold (0.0–1.0, bars below this are zeroed)
    #[arg(long)]
    noise_floor: Option<f32>,

    /// Enable monstercat smoothing (connects bar tops in a smooth curve)
    #[arg(long)]
    monstercat: bool,

    /// Temporal smoothing factor (0.0 = none, 0.9 = heavy)
    #[arg(long)]
    smoothing: Option<f32>,

    /// List available audio input devices
    #[arg(long)]
    list_devices: bool,
}

#[derive(Clone, PartialEq)]
enum Mode {
    Spectrum,
    Wave,
    Scope,
    Stereo,
}

impl Mode {
    fn from_str(s: &str) -> Self {
        match s {
            "wave" => Mode::Wave,
            "scope" => Mode::Scope,
            "stereo" => Mode::Stereo,
            _ => Mode::Spectrum,
        }
    }

    fn as_str(&self) -> &'static str {
        match self {
            Mode::Spectrum => "spectrum",
            Mode::Wave => "wave",
            Mode::Scope => "scope",
            Mode::Stereo => "stereo",
        }
    }
}

const DEFAULT_SAMPLE_RATE: u32 = 48000;
const MONSTERCAT_STRENGTH: f32 = 0.75;
const MIN_BARS: usize = 8;
const MAX_BARS: usize = 256;
const BAR_STEP: usize = 8;

fn start_audio(
    mono_buf: &audio::SampleBuffer,
    stereo: &audio::StereoPair,
    device: Option<&str>,
) -> Result<(u32, audio::CaptureHandle)> {
    if device == Some(audio::SYSTEM_AUDIO_LABEL) || device == Some("system") {
        audio::start_tap(
            mono_buf.clone(),
            (stereo.0.clone(), stereo.1.clone()),
            DEFAULT_SAMPLE_RATE,
        )
    } else {
        audio::start_capture(
            mono_buf.clone(),
            (stereo.0.clone(), stereo.1.clone()),
            device,
        )
    }
}

fn save_state(
    cfg: &mut config::Config,
    settings: &render::Settings,
    theme_name: &str,
    num_bars: usize,
    mode: &Mode,
) {
    cfg.smoothing = settings.smoothing;
    cfg.monstercat = settings.monstercat;
    cfg.noise_floor = settings.noise_floor;
    cfg.theme = theme_name.to_string();
    cfg.bars = num_bars;
    cfg.mode = mode.as_str().to_string();
    let _ = config::save(cfg);
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    if cli.list_devices {
        for name in audio::list_devices()? {
            println!("{}", name);
        }
        return Ok(());
    }

    // Load config, then let CLI args override
    let mut cfg = config::load();
    if let Some(ref m) = cli.mode {
        cfg.mode = m.clone();
    }
    if let Some(ref t) = cli.theme {
        cfg.theme = t.clone();
    }
    if let Some(f) = cli.fps {
        cfg.fps = f;
    }
    if let Some(b) = cli.bars {
        cfg.bars = b;
    }
    if let Some(f) = cli.low_freq {
        cfg.low_freq = f;
    }
    if let Some(f) = cli.high_freq {
        cfg.high_freq = f;
    }
    if let Some(s) = cli.smoothing {
        cfg.smoothing = s;
    }
    if cli.monstercat {
        cfg.monstercat = true;
    }
    if let Some(n) = cli.noise_floor {
        cfg.noise_floor = n;
    }

    let mode = Mode::from_str(&cfg.mode);

    // Start audio capture
    let mono_buf = audio::new_buffer(analysis::FFT_SIZE);
    let stereo = audio::new_stereo_buffers(analysis::FFT_SIZE);
    let mut device_name = cli
        .device
        .clone()
        .unwrap_or_else(|| "Default device".to_string());
    let (mut sample_rate, mut capture) =
        start_audio(&mono_buf, &stereo, cli.device.as_deref())?;

    // Init terminal
    let mut terminal = render::init()?;
    let fps = cfg.fps.max(1);
    let frame_duration = Duration::from_millis(1000 / fps);
    let mut num_bars = cfg.bars.clamp(MIN_BARS, MAX_BARS);
    let mut prev_bars: Vec<f32> = vec![0.0; num_bars];
    let mut prev_left: Vec<f32> = vec![0.0; num_bars];
    let mut prev_right: Vec<f32> = vec![0.0; num_bars];
    let low_freq = cfg.low_freq;
    let high_freq = cfg.high_freq;
    let mut theme_idx = theme::THEMES
        .iter()
        .position(|t| t.name == cfg.theme)
        .unwrap_or(0);
    let mut current_theme = &theme::THEMES[theme_idx];

    let mut settings = render::Settings {
        smoothing: cfg.smoothing.clamp(0.0, 0.99),
        monstercat: cfg.monstercat,
        noise_floor: cfg.noise_floor,
        theme_idx,
    };

    let mut autosens = analysis::AutoSensitivity::new();
    let mut autosens_l = analysis::AutoSensitivity::new();
    let mut autosens_r = analysis::AutoSensitivity::new();

    loop {
        let frame_start = Instant::now();

        match render::poll_input(Duration::ZERO)? {
            render::Action::Quit => break,
            render::Action::SelectDevice => {
                let devices = audio::list_devices()?;
                match render::device_menu(&mut terminal, &devices)? {
                    render::DeviceMenuResult::Selected(new_device) => {
                        drop(capture);
                        let (sr, handle) =
                            start_audio(&mono_buf, &stereo, new_device.as_deref())?;
                        sample_rate = sr;
                        capture = handle;
                        device_name =
                            new_device.unwrap_or_else(|| "Default device".to_string());
                        prev_bars = vec![0.0; num_bars];
                        prev_left = vec![0.0; num_bars];
                        prev_right = vec![0.0; num_bars];
                        autosens = analysis::AutoSensitivity::new();
                        autosens_l = analysis::AutoSensitivity::new();
                        autosens_r = analysis::AutoSensitivity::new();
                    }
                    render::DeviceMenuResult::Quit => break,
                    render::DeviceMenuResult::Cancelled => {}
                }
                continue;
            }
            render::Action::SelectTheme => {
                match render::theme_menu(&mut terminal, theme::THEMES, theme_idx)? {
                    render::ThemeMenuResult::Selected(idx) => {
                        theme_idx = idx;
                        current_theme = &theme::THEMES[idx];
                        settings.theme_idx = idx;
                        save_state(&mut cfg, &settings, current_theme.name, num_bars, &mode);
                    }
                    render::ThemeMenuResult::Quit => break,
                    render::ThemeMenuResult::Cancelled => {}
                }
                continue;
            }
            render::Action::Settings => {
                match render::settings_menu(&mut terminal, &settings, theme::THEMES)? {
                    Some(new_settings) => {
                        settings = new_settings;
                        theme_idx = settings.theme_idx;
                        current_theme = &theme::THEMES[theme_idx];
                        save_state(&mut cfg, &settings, current_theme.name, num_bars, &mode);
                    }
                    None => break,
                }
                continue;
            }
            render::Action::Help => {
                render::help(&mut terminal)?;
                continue;
            }
            render::Action::MoreBars => {
                num_bars = (num_bars + BAR_STEP).min(MAX_BARS);
                prev_bars = vec![0.0; num_bars];
                prev_left = vec![0.0; num_bars];
                prev_right = vec![0.0; num_bars];
                save_state(&mut cfg, &settings, current_theme.name, num_bars, &mode);
                continue;
            }
            render::Action::FewerBars => {
                num_bars = (num_bars.saturating_sub(BAR_STEP)).max(MIN_BARS);
                prev_bars = vec![0.0; num_bars];
                prev_left = vec![0.0; num_bars];
                prev_right = vec![0.0; num_bars];
                save_state(&mut cfg, &settings, current_theme.name, num_bars, &mode);
                continue;
            }
            render::Action::None => {}
        }

        match mode {
            Mode::Spectrum => {
                let samples = {
                    let buf = mono_buf.lock().unwrap();
                    buf.clone()
                };
                let magnitudes = analysis::spectrum(&samples);
                let bars = analysis::bin_spectrum(
                    &magnitudes, num_bars, sample_rate, low_freq, high_freq,
                );
                let mut smoothed = analysis::smooth(&prev_bars, &bars, settings.smoothing);
                if settings.monstercat {
                    analysis::monstercat(&mut smoothed, MONSTERCAT_STRENGTH);
                }
                if settings.noise_floor > 0.0 {
                    analysis::noise_gate(&mut smoothed, settings.noise_floor);
                }
                autosens.apply(&mut smoothed);
                render::draw_spectrum(&mut terminal, &smoothed, current_theme, &device_name)?;
                prev_bars = smoothed;
            }
            Mode::Stereo => {
                let left_samples = {
                    let buf = stereo.0.lock().unwrap();
                    buf.clone()
                };
                let right_samples = {
                    let buf = stereo.1.lock().unwrap();
                    buf.clone()
                };

                let left_mag = analysis::spectrum(&left_samples);
                let right_mag = analysis::spectrum(&right_samples);

                let left_bars = analysis::bin_spectrum(
                    &left_mag, num_bars, sample_rate, low_freq, high_freq,
                );
                let right_bars = analysis::bin_spectrum(
                    &right_mag, num_bars, sample_rate, low_freq, high_freq,
                );

                let mut smooth_l =
                    analysis::smooth(&prev_left, &left_bars, settings.smoothing);
                let mut smooth_r =
                    analysis::smooth(&prev_right, &right_bars, settings.smoothing);

                if settings.monstercat {
                    analysis::monstercat(&mut smooth_l, MONSTERCAT_STRENGTH);
                    analysis::monstercat(&mut smooth_r, MONSTERCAT_STRENGTH);
                }

                if settings.noise_floor > 0.0 {
                    analysis::noise_gate(&mut smooth_l, settings.noise_floor);
                    analysis::noise_gate(&mut smooth_r, settings.noise_floor);
                }

                autosens_l.apply(&mut smooth_l);
                autosens_r.apply(&mut smooth_r);

                render::draw_stereo(
                    &mut terminal, &smooth_l, &smooth_r, current_theme, &device_name,
                )?;

                prev_left = smooth_l;
                prev_right = smooth_r;
            }
            Mode::Wave => {
                let samples = {
                    let buf = mono_buf.lock().unwrap();
                    buf.clone()
                };
                render::draw_wave(&mut terminal, &samples, current_theme, &device_name)?;
            }
            Mode::Scope => {
                let samples = {
                    let buf = mono_buf.lock().unwrap();
                    buf.clone()
                };
                render::draw_scope(&mut terminal, &samples, current_theme, &device_name)?;
            }
        }

        let elapsed = frame_start.elapsed();
        if elapsed < frame_duration {
            std::thread::sleep(frame_duration - elapsed);
        }
    }

    render::cleanup(&mut terminal)?;
    Ok(())
}
