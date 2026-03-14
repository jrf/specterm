mod analysis;
mod audio;
mod render;

use anyhow::Result;
use clap::Parser;

#[derive(Parser)]
#[command(name = "vox", about = "Terminal audio visualizer")]
struct Cli {
    /// Visualization mode
    #[arg(short, long, default_value = "spectrum")]
    mode: Mode,

    /// Audio input device (defaults to system default)
    #[arg(short, long)]
    device: Option<String>,
}

#[derive(Clone, clap::ValueEnum)]
enum Mode {
    /// Spectrum analyzer (frequency bars)
    Spectrum,
    /// Waveform (amplitude over time)
    Wave,
    /// Oscilloscope (triggered waveform)
    Scope,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // TODO: wire up audio capture -> analysis -> render pipeline
    // 1. Start audio capture thread (audio::capture)
    // 2. Run render loop reading from shared buffer (analysis::process + render::draw)

    println!(
        "vox: mode={}",
        match cli.mode {
            Mode::Spectrum => "spectrum",
            Mode::Wave => "wave",
            Mode::Scope => "scope",
        }
    );

    Ok(())
}
