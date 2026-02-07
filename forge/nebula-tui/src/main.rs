mod aurora_client;
mod brain_client;
mod commands;
mod telemetry;
mod ui;

use std::io::{self, Write};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyModifiers},
    terminal::{self, disable_raw_mode, enable_raw_mode, Clear, ClearType},
    ExecutableCommand,
};
use ratatui::prelude::*;
use ratatui::{TerminalOptions, Viewport};

/// Rich output block for the display.
#[derive(Clone, Debug)]
pub enum OutputBlock {
    /// Plain text line.
    Text(String),
    /// Styled text with color.
    Styled { text: String, color: ui::BlockColor },
    /// Inline widget box with border.
    Widget {
        title: String,
        lines: Vec<String>,
        color: ui::BlockColor,
    },
    /// Visual separator.
    Separator,
}

/// Application state.
pub struct App {
    /// Current input in the omni-bar.
    pub input: String,
    /// Cursor position in input.
    pub cursor: usize,
    /// Output blocks (scrollable history).
    pub output: Vec<OutputBlock>,
    /// Scroll offset for output (0 = bottom).
    pub scroll: u16,
    /// System telemetry snapshot.
    pub telemetry: telemetry::SysTelemetry,
    /// Aurora AI status.
    pub aurora: aurora_client::AuroraStatus,
    /// Whether we should quit.
    pub quit: bool,
    /// Command history.
    pub history: Vec<String>,
    /// Current position in history.
    pub history_pos: Option<usize>,
    /// Whether a brain query is in progress.
    pub thinking: bool,
    /// Thinking animation frame counter.
    pub thinking_frame: u8,
    /// Receiver for brain responses.
    pub brain_rx: mpsc::Receiver<brain_client::BrainResponse>,
    /// Sender for brain responses (cloned into threads).
    pub brain_tx: mpsc::Sender<brain_client::BrainResponse>,
}

impl App {
    fn new() -> Self {
        let (tx, rx) = mpsc::channel();
        let mut app = Self {
            input: String::new(),
            cursor: 0,
            output: vec![
                OutputBlock::Styled {
                    text: "AETHER OS v0.3 \u{2014} AI-Native Operating System".into(),
                    color: ui::BlockColor::Cyan,
                },
                OutputBlock::Text("Type anything. The OS understands you.".into()),
                OutputBlock::Text("Use !command for shell passthrough, 'help' for more.".into()),
                OutputBlock::Separator,
            ],
            scroll: 0,
            telemetry: telemetry::SysTelemetry::default(),
            aurora: aurora_client::AuroraStatus::default(),
            quit: false,
            history: Vec::new(),
            history_pos: None,
            thinking: false,
            thinking_frame: 0,
            brain_rx: rx,
            brain_tx: tx,
        };
        app.telemetry = telemetry::read_telemetry();
        app.aurora = aurora_client::check_health();
        app
    }

    fn push_text(&mut self, line: &str) {
        for l in line.lines() {
            self.output.push(OutputBlock::Text(l.to_string()));
        }
    }

    fn push_styled(&mut self, text: &str, color: ui::BlockColor) {
        self.output.push(OutputBlock::Styled {
            text: text.to_string(),
            color,
        });
    }

    fn push_brain_response(&mut self, resp: brain_client::BrainResponse) {
        // Add text
        if !resp.text.is_empty() {
            for line in resp.text.lines() {
                self.output.push(OutputBlock::Text(line.to_string()));
            }
        }

        // Add widgets
        for widget in &resp.widgets {
            let color = match widget.widget_type.as_str() {
                "weather" => ui::BlockColor::Yellow,
                "system" => ui::BlockColor::Green,
                "file" => ui::BlockColor::Blue,
                "table" => ui::BlockColor::Cyan,
                _ => ui::BlockColor::White,
            };
            self.output.push(OutputBlock::Widget {
                title: widget.title.clone(),
                lines: widget.lines.clone(),
                color,
            });
        }

        if resp.latency_ms > 0 {
            self.output.push(OutputBlock::Styled {
                text: format!("  [{:.1}s]", resp.latency_ms as f64 / 1000.0),
                color: ui::BlockColor::DarkGray,
            });
        }

        self.output.push(OutputBlock::Separator);
    }

    fn submit_command(&mut self) {
        let cmd = self.input.trim().to_string();
        if cmd.is_empty() {
            return;
        }

        self.push_styled(&format!("> {}", cmd), ui::BlockColor::Cyan);
        self.history.push(cmd.clone());
        self.history_pos = None;
        self.input.clear();
        self.cursor = 0;
        self.scroll = 0;

        // Local-only commands
        let lower = cmd.to_lowercase();
        match lower.as_str() {
            "help" => {
                let result = commands::help_text();
                self.push_text(&result);
                self.output.push(OutputBlock::Separator);
                return;
            }
            "clear" => {
                self.output.clear();
                return;
            }
            "exit" | "quit" => {
                self.quit = true;
                return;
            }
            "sysinfo" => {
                self.telemetry = telemetry::read_telemetry();
                let result = commands::sysinfo_text(&self.telemetry);
                self.push_text(&result);
                self.output.push(OutputBlock::Separator);
                return;
            }
            _ => {}
        }

        // Shell passthrough with ! prefix
        if cmd.starts_with('!') {
            let shell_cmd = &cmd[1..];
            let result = commands::run_shell(shell_cmd);
            self.push_text(&result);
            self.output.push(OutputBlock::Separator);
            return;
        }

        // Everything else goes to brain (async)
        self.thinking = true;
        self.thinking_frame = 0;
        let tx = self.brain_tx.clone();
        let input = cmd.clone();
        std::thread::spawn(move || {
            let result = match brain_client::query_brain(&input) {
                Ok(resp) => resp,
                Err(e) => brain_client::BrainResponse {
                    ok: false,
                    text: format!("Brain error: {}", e),
                    widgets: vec![],
                    latency_ms: 0,
                    error: Some(e),
                },
            };
            let _ = tx.send(result);
        });
    }
}

/// Try to get terminal size, with fallback for serial consoles.
fn get_terminal_size() -> (u16, u16) {
    if let Ok((w, h)) = terminal::size() {
        if w > 0 && h > 0 {
            return (w, h);
        }
    }
    let cols = std::env::var("COLUMNS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(80u16);
    let rows = std::env::var("LINES")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(24u16);
    (cols, rows)
}

fn main() -> io::Result<()> {
    let (cols, rows) = get_terminal_size();

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    stdout.execute(Clear(ClearType::All))?;
    stdout.execute(cursor::MoveTo(0, 0))?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::with_options(
        backend,
        TerminalOptions {
            viewport: Viewport::Fixed(Rect::new(0, 0, cols, rows)),
        },
    )?;

    let mut app = App::new();
    let tick_rate = Duration::from_millis(250); // Faster tick for thinking animation
    let mut last_tick = Instant::now();
    let mut telemetry_interval = Instant::now();

    loop {
        terminal.draw(|f| ui::draw(f, &app))?;

        let timeout = tick_rate.saturating_sub(last_tick.elapsed());
        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                match (key.modifiers, key.code) {
                    (KeyModifiers::CONTROL, KeyCode::Char('c')) => {
                        app.quit = true;
                    }
                    (_, KeyCode::Enter) => {
                        if !app.thinking {
                            app.submit_command();
                        }
                    }
                    (_, KeyCode::Backspace) => {
                        if app.cursor > 0 && !app.thinking {
                            app.cursor -= 1;
                            app.input.remove(app.cursor);
                        }
                    }
                    (_, KeyCode::Delete) => {
                        if app.cursor < app.input.len() && !app.thinking {
                            app.input.remove(app.cursor);
                        }
                    }
                    (_, KeyCode::Left) => {
                        if app.cursor > 0 {
                            app.cursor -= 1;
                        }
                    }
                    (_, KeyCode::Right) => {
                        if app.cursor < app.input.len() {
                            app.cursor += 1;
                        }
                    }
                    (_, KeyCode::Home) => {
                        app.cursor = 0;
                    }
                    (_, KeyCode::End) => {
                        app.cursor = app.input.len();
                    }
                    (_, KeyCode::Up) => {
                        if !app.history.is_empty() {
                            let pos = match app.history_pos {
                                Some(p) if p > 0 => p - 1,
                                Some(p) => p,
                                None => app.history.len() - 1,
                            };
                            app.history_pos = Some(pos);
                            app.input = app.history[pos].clone();
                            app.cursor = app.input.len();
                        }
                    }
                    (_, KeyCode::Down) => {
                        if let Some(pos) = app.history_pos {
                            if pos + 1 < app.history.len() {
                                let new_pos = pos + 1;
                                app.history_pos = Some(new_pos);
                                app.input = app.history[new_pos].clone();
                                app.cursor = app.input.len();
                            } else {
                                app.history_pos = None;
                                app.input.clear();
                                app.cursor = 0;
                            }
                        }
                    }
                    (_, KeyCode::PageUp) => {
                        app.scroll = app.scroll.saturating_add(10);
                    }
                    (_, KeyCode::PageDown) => {
                        app.scroll = app.scroll.saturating_sub(10);
                    }
                    (_, KeyCode::Char(c)) => {
                        if !app.thinking {
                            app.input.insert(app.cursor, c);
                            app.cursor += 1;
                        }
                    }
                    _ => {}
                }
            }
        }

        // Check for brain response
        if app.thinking {
            if let Ok(resp) = app.brain_rx.try_recv() {
                app.thinking = false;
                app.push_brain_response(resp);
            } else {
                app.thinking_frame = app.thinking_frame.wrapping_add(1);
            }
        }

        // Periodic tick
        if last_tick.elapsed() >= tick_rate {
            last_tick = Instant::now();
        }

        // Telemetry refresh every 2 seconds
        if telemetry_interval.elapsed() >= Duration::from_secs(2) {
            app.telemetry = telemetry::read_telemetry();
            telemetry_interval = Instant::now();
        }

        if app.quit {
            break;
        }
    }

    disable_raw_mode()?;
    let mut stdout = io::stdout();
    stdout.execute(Clear(ClearType::All))?;
    stdout.execute(cursor::MoveTo(0, 0))?;
    stdout.flush()?;
    Ok(())
}
