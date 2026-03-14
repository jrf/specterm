//! Terminal rendering using ratatui + crossterm.

use crate::theme::Theme;

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use ratatui::{
    backend::CrosstermBackend,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Bar, BarChart, BarGroup, Block, Borders, List, ListItem, Padding},
    widgets::canvas::{Canvas, Line as CanvasLine},
    Terminal,
};
use std::time::Duration;

pub type Term = Terminal<CrosstermBackend<std::io::Stdout>>;

/// Initialize the terminal for raw-mode rendering.
pub fn init() -> Result<Term> {
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
pub fn cleanup(terminal: &mut Term) -> Result<()> {
    crossterm::terminal::disable_raw_mode()?;
    crossterm::execute!(
        terminal.backend_mut(),
        crossterm::terminal::LeaveAlternateScreen,
        crossterm::cursor::Show,
    )?;
    Ok(())
}

/// Action returned from input polling.
pub enum Action {
    None,
    Quit,
    SelectDevice,
    SelectTheme,
    Settings,
    Help,
    MoreBars,
    FewerBars,
}

/// Poll for input events. Returns the action to take.
pub fn poll_input(timeout: Duration) -> Result<Action> {
    if event::poll(timeout)? {
        if let Event::Key(key) = event::read()? {
            if matches!(key.code, KeyCode::Char('q') | KeyCode::Esc)
                || (key.code == KeyCode::Char('c')
                    && key.modifiers.contains(KeyModifiers::CONTROL))
            {
                return Ok(Action::Quit);
            }
            match key.code {
                KeyCode::Char('d') => return Ok(Action::SelectDevice),
                KeyCode::Char('t') => return Ok(Action::SelectTheme),
                KeyCode::Char('s') => return Ok(Action::Settings),
                KeyCode::Char('?') => return Ok(Action::Help),
                KeyCode::Up | KeyCode::Char('+') => return Ok(Action::MoreBars),
                KeyCode::Down | KeyCode::Char('-') => return Ok(Action::FewerBars),
                _ => {}
            }
        }
    }
    Ok(Action::None)
}

/// Result of the device menu interaction.
pub enum DeviceMenuResult {
    /// User selected a device (None = default device).
    Selected(Option<String>),
    /// User cancelled (Esc/d) — go back to visualizer.
    Cancelled,
    /// User wants to quit entirely.
    Quit,
}

/// Show an interactive device selection menu.
pub fn device_menu(terminal: &mut Term, devices: &[String]) -> Result<DeviceMenuResult> {
    let mut selected: usize = 0;
    let total = devices.len() + 1;

    loop {
        terminal.draw(|frame| {
            let area = frame.area();

            let items: Vec<ListItem> = std::iter::once(ListItem::new(Line::from(vec![
                Span::raw("  Default device"),
            ])))
            .chain(devices.iter().map(|name| {
                ListItem::new(Line::from(vec![Span::raw(format!("  {}", name))]))
            }))
            .enumerate()
            .map(|(i, item)| {
                if i == selected {
                    item.style(
                        Style::default()
                            .fg(Color::Black)
                            .bg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    )
                } else {
                    item
                }
            })
            .collect();

            let list = List::new(items).block(
                Block::default()
                    .title(" sonitus — select audio device ")
                    .title_bottom(" ↑/↓ navigate  Enter select  Esc back  q quit ")
                    .borders(Borders::ALL)
                    .padding(Padding::vertical(1)),
            );

            frame.render_widget(list, area);
        })?;

        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Up | KeyCode::Char('k') => {
                        selected = selected.saturating_sub(1);
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        if selected + 1 < total {
                            selected += 1;
                        }
                    }
                    KeyCode::Enter => {
                        return if selected == 0 {
                            Ok(DeviceMenuResult::Selected(None))
                        } else {
                            Ok(DeviceMenuResult::Selected(Some(
                                devices[selected - 1].clone(),
                            )))
                        };
                    }
                    KeyCode::Esc | KeyCode::Char('d') => {
                        return Ok(DeviceMenuResult::Cancelled);
                    }
                    KeyCode::Char('q') => {
                        return Ok(DeviceMenuResult::Quit);
                    }
                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        return Ok(DeviceMenuResult::Quit);
                    }
                    _ => {}
                }
            }
        }
    }
}

/// Result of the theme menu interaction.
pub enum ThemeMenuResult {
    Selected(usize),
    Cancelled,
    Quit,
}

/// Show an interactive theme selection menu with preview swatches.
pub fn theme_menu(terminal: &mut Term, themes: &[Theme], current_idx: usize) -> Result<ThemeMenuResult> {
    let mut selected = current_idx;
    let total = themes.len();

    loop {
        terminal.draw(|frame| {
            let area = frame.area();

            let items: Vec<ListItem> = themes
                .iter()
                .enumerate()
                .map(|(i, theme)| {
                    // Build a swatch showing the gradient colors
                    let mut spans: Vec<Span> = vec![Span::raw("  ")];
                    for &color in theme.gradient {
                        spans.push(Span::styled("██", Style::default().fg(color)));
                    }
                    spans.push(Span::raw(format!("  {}", theme.name)));

                    let item = ListItem::new(Line::from(spans));
                    if i == selected {
                        item.style(
                            Style::default()
                                .bg(Color::Rgb(40, 40, 40))
                                .add_modifier(Modifier::BOLD),
                        )
                    } else {
                        item
                    }
                })
                .collect();

            let list = List::new(items).block(
                Block::default()
                    .title(" sonitus — select theme ")
                    .title_bottom(" ↑/↓ navigate  Enter select  Esc back  q quit ")
                    .borders(Borders::ALL)
                    .padding(Padding::vertical(1)),
            );

            frame.render_widget(list, area);
        })?;

        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Up | KeyCode::Char('k') => {
                        selected = selected.saturating_sub(1);
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        if selected + 1 < total {
                            selected += 1;
                        }
                    }
                    KeyCode::Enter => {
                        return Ok(ThemeMenuResult::Selected(selected));
                    }
                    KeyCode::Esc | KeyCode::Char('t') => {
                        return Ok(ThemeMenuResult::Cancelled);
                    }
                    KeyCode::Char('q') => {
                        return Ok(ThemeMenuResult::Quit);
                    }
                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        return Ok(ThemeMenuResult::Quit);
                    }
                    _ => {}
                }
            }
        }
    }
}

/// Mutable settings that can be changed at runtime.
#[derive(Clone)]
pub struct Settings {
    pub smoothing: f32,
    pub monstercat: bool,
    pub noise_floor: f32,
    pub theme_idx: usize,
    /// Number of distinct gradient colors (0 = all).
    pub colors: usize,
}

/// Show settings menu. Returns updated settings.
pub fn settings_menu(terminal: &mut Term, settings: &Settings, themes: &[Theme]) -> Result<Option<Settings>> {
    let mut current = settings.clone();
    let mut selected: usize = 0;
    let num_items = 5;

    loop {
        let theme = &themes[current.theme_idx.min(themes.len() - 1)];

        terminal.draw(|frame| {
            let area = frame.area();

            let smoothing_bar = slider_bar(current.smoothing, 0.0, 0.99, 20);
            let noise_bar = slider_bar(current.noise_floor, 0.0, 0.05, 20);

            // Theme swatch
            let mut theme_spans: Vec<Span> = vec![Span::styled(
                format!("  {:16}", "Theme"),
                Style::default().fg(Color::Cyan),
            )];
            for &color in theme.gradient {
                theme_spans.push(Span::styled("██", Style::default().fg(color)));
            }
            theme_spans.push(Span::raw(format!("  {}", theme.name)));

            let items: Vec<ListItem> = vec![
                ListItem::new(Line::from(theme_spans)),
                ListItem::new(Line::from(vec![
                    Span::styled(
                        format!("  {:16}", "Smoothing"),
                        Style::default().fg(Color::Cyan),
                    ),
                    Span::raw(format!("{} {:.2}", smoothing_bar, current.smoothing)),
                ])),
                ListItem::new(Line::from(vec![
                    Span::styled(
                        format!("  {:16}", "Monstercat"),
                        Style::default().fg(Color::Cyan),
                    ),
                    Span::raw(if current.monstercat { "[ON]" } else { "[OFF]" }),
                ])),
                ListItem::new(Line::from(vec![
                    Span::styled(
                        format!("  {:16}", "Noise floor"),
                        Style::default().fg(Color::Cyan),
                    ),
                    Span::raw(format!("{} {:.4}", noise_bar, current.noise_floor)),
                ])),
                ListItem::new(Line::from(vec![
                    Span::styled(
                        format!("  {:16}", "Colors"),
                        Style::default().fg(Color::Cyan),
                    ),
                    Span::raw(if current.colors == 0 {
                        "all".to_string()
                    } else {
                        current.colors.to_string()
                    }),
                ])),
            ]
            .into_iter()
            .enumerate()
            .map(|(i, item)| {
                if i == selected {
                    item.style(
                        Style::default()
                            .bg(Color::Rgb(40, 40, 40))
                            .add_modifier(Modifier::BOLD),
                    )
                } else {
                    item
                }
            })
            .collect();

            let list = List::new(items).block(
                Block::default()
                    .title(" sonitus — settings ")
                    .title_bottom(" ↑/↓ navigate  ←/→ adjust  Enter/Space toggle  Esc back ")
                    .borders(Borders::ALL)
                    .padding(Padding::new(2, 2, 1, 1)),
            );

            frame.render_widget(list, area);
        })?;

        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Up | KeyCode::Char('k') => {
                        selected = selected.saturating_sub(1);
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        if selected + 1 < num_items {
                            selected += 1;
                        }
                    }
                    KeyCode::Left | KeyCode::Char('h') => {
                        adjust_setting(&mut current, selected, -1, themes.len());
                    }
                    KeyCode::Right | KeyCode::Char('l') => {
                        adjust_setting(&mut current, selected, 1, themes.len());
                    }
                    KeyCode::Enter | KeyCode::Char(' ') => {
                        if selected == 2 {
                            current.monstercat = !current.monstercat;
                        }
                    }
                    KeyCode::Esc | KeyCode::Char('s') => {
                        return Ok(Some(current));
                    }
                    KeyCode::Char('q') => {
                        return Ok(None);
                    }
                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        return Ok(None);
                    }
                    _ => {}
                }
            }
        }
    }
}

fn adjust_setting(settings: &mut Settings, idx: usize, direction: i32, num_themes: usize) {
    match idx {
        0 => {
            // Theme: cycle through themes
            if direction > 0 {
                settings.theme_idx = (settings.theme_idx + 1) % num_themes;
            } else if settings.theme_idx == 0 {
                settings.theme_idx = num_themes - 1;
            } else {
                settings.theme_idx -= 1;
            }
        }
        1 => {
            // Smoothing: step by 0.05
            settings.smoothing = (settings.smoothing + direction as f32 * 0.05).clamp(0.0, 0.99);
        }
        3 => {
            // Noise floor: step by 0.002
            settings.noise_floor =
                (settings.noise_floor + direction as f32 * 0.002).clamp(0.0, 0.05);
        }
        4 => {
            // Colors: 0 (all), 2..=16
            if direction > 0 {
                settings.colors = match settings.colors {
                    0 => 2,
                    c if c < 16 => c + 1,
                    _ => 0,
                };
            } else {
                settings.colors = match settings.colors {
                    0 => 16,
                    2 => 0,
                    c => c - 1,
                };
            }
        }
        _ => {}
    }
}

fn slider_bar(value: f32, min: f32, max: f32, width: usize) -> String {
    let ratio = ((value - min) / (max - min)).clamp(0.0, 1.0);
    let filled = (ratio * width as f32) as usize;
    let empty = width - filled;
    format!("[{}{}]", "█".repeat(filled), "░".repeat(empty))
}

/// Show help overlay. Blocks until any key is pressed.
pub fn help(terminal: &mut Term) -> Result<()> {
    let bindings = [
        ("?", "Show this help"),
        ("d", "Select audio device"),
        ("t", "Select color theme"),
        ("s", "Settings (smoothing, monstercat, noise)"),
        ("Up / +", "More bars"),
        ("Down / -", "Fewer bars"),
        ("q / Esc", "Quit"),
        ("Ctrl+C", "Quit"),
    ];

    let modes = [
        ("--mode spectrum", "Frequency spectrum bars (default)"),
        ("--mode wave", "Waveform amplitude plot"),
        ("--mode scope", "Oscilloscope (triggered waveform)"),
        ("--fps N", "Set target framerate (default: 60)"),
        ("--low-freq N", "Low frequency cutoff in Hz (default: 20)"),
        ("--high-freq N", "High frequency cutoff in Hz (default: 20000)"),
        ("--noise-floor N", "Noise gate threshold (default: 0.0)"),
    ];

    terminal.draw(|frame| {
        let area = frame.area();

        let mut lines: Vec<Line> = Vec::new();

        lines.push(Line::from(Span::styled(
            "Keybindings",
            Style::default().add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(""));
        for (key, desc) in &bindings {
            lines.push(Line::from(vec![
                Span::styled(format!("  {:12}", key), Style::default().fg(Color::Cyan)),
                Span::raw(*desc),
            ]));
        }

        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Modes",
            Style::default().add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(""));
        for (flag, desc) in &modes {
            lines.push(Line::from(vec![
                Span::styled(format!("  {:20}", flag), Style::default().fg(Color::Cyan)),
                Span::raw(*desc),
            ]));
        }

        let paragraph = ratatui::widgets::Paragraph::new(lines).block(
            Block::default()
                .title(" sonitus — help ")
                .title_bottom(" press any key to close ")
                .borders(Borders::ALL)
                .padding(Padding::new(2, 2, 1, 1)),
        );

        frame.render_widget(paragraph, area);
    })?;

    // Wait for any key press
    loop {
        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(_) = event::read()? {
                return Ok(());
            }
        }
    }
}

/// Draw spectrum bars. Bars are pre-normalized by AutoSensitivity (0.0–1.0+ range).
pub fn draw_spectrum(terminal: &mut Term, bars: &[f32], theme: &Theme, device: &str, num_colors: usize) -> Result<()> {
    let theme_name = theme.name;
    let num_bars = bars.len();
    terminal.draw(|frame| {
        let area = frame.area();
        let max_val = bars.iter().cloned().fold(0.0f32, f32::max).max(0.001);

        let bar_width = ((area.width as usize).saturating_sub(2) / bars.len().max(1)).max(1) as u16;

        let ratatui_bars: Vec<Bar> = bars
            .iter()
            .map(|&v| {
                let normalized = v / max_val;
                let height = (normalized * 100.0) as u64;
                Bar::default()
                    .value(height)
                    .text_value(String::new())
                    .style(Style::default().fg(theme.bar_color(normalized, num_colors)))
            })
            .collect();

        let chart = BarChart::default()
            .block(
                Block::default()
                    .title(format!(" sonitus — spectrum [{}] ({} bars) ", theme_name, num_bars))
                    .title_bottom(format!(" {} | ? help ", device))
                    .borders(Borders::ALL),
            )
            .data(BarGroup::default().bars(&ratatui_bars))
            .bar_width(bar_width)
            .bar_gap(0)
            .max(100);

        frame.render_widget(chart, area);
    })?;

    Ok(())
}

/// Draw waveform.
pub fn draw_wave(terminal: &mut Term, samples: &[f32], theme: &Theme, device: &str) -> Result<()> {
    let color = theme.wave_color;
    let title = " sonitus — waveform ";
    let bottom = format!(" {} | ? help ", device);
    draw_wave_inner(terminal, samples, title, &bottom, color)
}

/// Draw oscilloscope (zero-crossing triggered waveform).
pub fn draw_scope(terminal: &mut Term, samples: &[f32], theme: &Theme, device: &str) -> Result<()> {
    let trigger_offset = samples
        .windows(2)
        .position(|w| w[0] <= 0.0 && w[1] > 0.0)
        .unwrap_or(0);

    let triggered = &samples[trigger_offset..];
    let bottom = format!(" {} | ? help ", device);
    draw_wave_inner(terminal, triggered, " sonitus — oscilloscope ", &bottom, theme.scope_color)
}

fn draw_wave_inner(terminal: &mut Term, samples: &[f32], title: &str, bottom: &str, color: Color) -> Result<()> {
    terminal.draw(|frame| {
        let area = frame.area();
        let inner = Rect::new(area.x + 1, area.y + 1, area.width.saturating_sub(2), area.height.saturating_sub(2));

        let canvas = Canvas::default()
            .block(Block::default().title(title).title_bottom(bottom).borders(Borders::ALL))
            .x_bounds([0.0, inner.width as f64])
            .y_bounds([-1.0, 1.0])
            .paint(|ctx| {
                if samples.len() < 2 {
                    return;
                }
                let step = samples.len() as f64 / inner.width as f64;
                for i in 0..inner.width.saturating_sub(1) as usize {
                    let idx0 = (i as f64 * step) as usize;
                    let idx1 = ((i + 1) as f64 * step) as usize;
                    let y0 = samples.get(idx0).copied().unwrap_or(0.0) as f64;
                    let y1 = samples.get(idx1).copied().unwrap_or(0.0) as f64;
                    ctx.draw(&CanvasLine {
                        x1: i as f64,
                        y1: y0,
                        x2: (i + 1) as f64,
                        y2: y1,
                        color,
                    });
                }
            });

        frame.render_widget(canvas, area);
    })?;

    Ok(())
}

/// Draw stereo spectrum: left channel bars grow up from center, right channel grows down.
pub fn draw_stereo(
    terminal: &mut Term,
    left_bars: &[f32],
    right_bars: &[f32],
    theme: &Theme,
    device: &str,
    num_colors: usize,
) -> Result<()> {
    let theme_name = theme.name;
    let num_bars = left_bars.len();

    terminal.draw(|frame| {
        let area = frame.area();
        let inner_w = area.width.saturating_sub(2) as usize;
        let inner_h = area.height.saturating_sub(2) as f64;
        let half_h = inner_h / 2.0;

        let left_max = left_bars.iter().cloned().fold(0.0f32, f32::max).max(0.001);
        let right_max = right_bars.iter().cloned().fold(0.0f32, f32::max).max(0.001);

        let canvas = Canvas::default()
            .block(
                Block::default()
                    .title(format!(" sonitus — stereo [{}] ({} bars) ", theme_name, num_bars))
                    .title_bottom(format!(" {} | ? help ", device))
                    .borders(Borders::ALL),
            )
            .x_bounds([0.0, inner_w as f64])
            .y_bounds([-half_h, half_h])
            .paint(|ctx| {
                let bar_w = (inner_w as f64 / left_bars.len() as f64).max(1.0);

                // Left channel: bars grow upward from center (y=0)
                for (i, &v) in left_bars.iter().enumerate() {
                    let normalized = (v / left_max).clamp(0.0, 1.0);
                    let height = normalized as f64 * half_h;
                    let x = i as f64 * bar_w;
                    let color = theme.bar_color(normalized, num_colors);

                    // Draw bar as vertical lines
                    let steps = (height * 2.0).max(1.0) as usize;
                    for s in 0..steps {
                        let y = s as f64 / steps as f64 * height;
                        ctx.draw(&CanvasLine {
                            x1: x,
                            y1: y,
                            x2: x + bar_w - 0.5,
                            y2: y,
                            color,
                        });
                    }
                }

                // Right channel: bars grow downward from center (y=0)
                for (i, &v) in right_bars.iter().enumerate() {
                    let normalized = (v / right_max).clamp(0.0, 1.0);
                    let height = normalized as f64 * half_h;
                    let x = i as f64 * bar_w;
                    let color = theme.bar_color(normalized, num_colors);

                    let steps = (height * 2.0).max(1.0) as usize;
                    for s in 0..steps {
                        let y = -(s as f64 / steps as f64 * height);
                        ctx.draw(&CanvasLine {
                            x1: x,
                            y1: y,
                            x2: x + bar_w - 0.5,
                            y2: y,
                            color,
                        });
                    }
                }

                // Center line
                ctx.draw(&CanvasLine {
                    x1: 0.0,
                    y1: 0.0,
                    x2: inner_w as f64,
                    y2: 0.0,
                    color: Color::DarkGray,
                });
            });

        frame.render_widget(canvas, area);
    })?;

    Ok(())
}
