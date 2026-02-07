mod aurora_client;
mod commands;
mod telemetry;
mod ui;

use std::io;
use std::time::{Duration, Instant};

use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::prelude::*;

/// Application state.
pub struct App {
    /// Current input in the omni-bar.
    pub input: String,
    /// Cursor position in input.
    pub cursor: usize,
    /// Output lines (scrollable history).
    pub output: Vec<String>,
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
    /// Current position in history (for up/down navigation).
    pub history_pos: Option<usize>,
}

impl App {
    fn new() -> Self {
        let mut app = Self {
            input: String::new(),
            cursor: 0,
            output: vec![
                "Welcome to Nebula — AetherOS TUI Shell v0.3".into(),
                "Type 'help' for commands, Ctrl-C to exit.".into(),
                String::new(),
            ],
            scroll: 0,
            telemetry: telemetry::SysTelemetry::default(),
            aurora: aurora_client::AuroraStatus::default(),
            quit: false,
            history: Vec::new(),
            history_pos: None,
        };
        app.telemetry = telemetry::read_telemetry();
        app.aurora = aurora_client::check_health();
        app
    }

    fn push_output(&mut self, line: &str) {
        for l in line.lines() {
            self.output.push(l.to_string());
        }
    }

    fn submit_command(&mut self) {
        let cmd = self.input.trim().to_string();
        if cmd.is_empty() {
            return;
        }

        self.push_output(&format!("> {}", cmd));
        self.history.push(cmd.clone());
        self.history_pos = None;
        self.input.clear();
        self.cursor = 0;
        self.scroll = 0;

        let result = commands::execute(&cmd, &self.telemetry, &self.aurora);
        if result == "__CLEAR__" {
            self.output.clear();
        } else if result == "__QUIT__" {
            self.quit = true;
        } else {
            self.push_output(&result);
            self.push_output("");
        }

        // Refresh telemetry + aurora after command
        self.telemetry = telemetry::read_telemetry();
        self.aurora = aurora_client::check_health();
    }
}

fn main() -> io::Result<()> {
    // Set up terminal
    enable_raw_mode()?;
    io::stdout().execute(EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;

    let mut app = App::new();
    let tick_rate = Duration::from_millis(500);
    let mut last_tick = Instant::now();

    // Main loop
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
                        app.submit_command();
                    }
                    (_, KeyCode::Backspace) => {
                        if app.cursor > 0 {
                            app.cursor -= 1;
                            app.input.remove(app.cursor);
                        }
                    }
                    (_, KeyCode::Delete) => {
                        if app.cursor < app.input.len() {
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
                        app.input.insert(app.cursor, c);
                        app.cursor += 1;
                    }
                    _ => {}
                }
            }
        }

        // Periodic telemetry refresh
        if last_tick.elapsed() >= tick_rate {
            app.telemetry = telemetry::read_telemetry();
            last_tick = Instant::now();
        }

        if app.quit {
            break;
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    io::stdout().execute(LeaveAlternateScreen)?;
    Ok(())
}
