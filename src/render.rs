//! Terminal rendering using ratatui + crossterm.

use anyhow::Result;
use ratatui::{
    backend::CrosstermBackend,
    Terminal,
};

/// Initialize the terminal for raw-mode rendering.
pub fn init() -> Result<Terminal<CrosstermBackend<std::io::Stdout>>> {
    crossterm::terminal::enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    crossterm::execute!(
        stdout,
        crossterm::terminal::EnterAlternateScreen,
        crossterm::cursor::Hide,
    )?;
    let backend = CrosstermBackend::new(stdout);
    let terminal = Terminal::new(backend)?;
    Ok(terminal)
}

/// Restore the terminal to normal mode.
pub fn cleanup(mut terminal: Terminal<CrosstermBackend<std::io::Stdout>>) -> Result<()> {
    crossterm::terminal::disable_raw_mode()?;
    crossterm::execute!(
        terminal.backend_mut(),
        crossterm::terminal::LeaveAlternateScreen,
        crossterm::cursor::Show,
    )?;
    Ok(())
}

/// Draw spectrum bars.
pub fn draw_spectrum(
    _terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    _bars: &[f32],
) -> Result<()> {
    // TODO:
    // - Map bar values to terminal row heights
    // - Use ratatui BarChart or custom canvas
    // - Color gradient from bottom (cool) to top (hot)
    todo!("spectrum rendering not yet implemented")
}

/// Draw waveform.
pub fn draw_wave(
    _terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    _samples: &[f32],
) -> Result<()> {
    // TODO:
    // - Plot amplitude as a line across terminal width
    // - Use braille characters for sub-cell resolution
    todo!("waveform rendering not yet implemented")
}
