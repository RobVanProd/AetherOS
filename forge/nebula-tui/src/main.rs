mod aurora_client;
mod brain_client;
mod commands;
mod context;
mod feed;
mod input;
mod proactive;
mod tasks;
mod telemetry;
mod ui;
mod widgets;

use std::io::{self, Write};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use crossterm::{
    cursor,
    event::{self, Event},
    terminal::{self, disable_raw_mode, enable_raw_mode, Clear, ClearType},
    ExecutableCommand,
};
use ratatui::prelude::*;
use ratatui::{TerminalOptions, Viewport};

use feed::{FeedItem, FeedSource, FeedStore, Priority, WidgetData};
use input::AppAction;
use ui::ActivePanel;

/// Application state.
pub struct App {
    /// Current input in the omni-bar.
    pub input: String,
    /// Cursor position in input.
    pub cursor: usize,
    /// Feed store (replaces old output Vec).
    pub feed: FeedStore,
    /// Scroll offset for feed (0 = bottom).
    pub feed_scroll: u16,
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
    /// Which panel currently has focus.
    pub active_panel: ActivePanel,
    /// Selected feed item index (within visible items).
    pub selected_feed_item: Option<usize>,
    /// Receiver for proactive feed items from background sources.
    pub proactive_rx: mpsc::Receiver<FeedItem>,
    /// Sender for proactive feed items (cloned into background threads).
    pub proactive_tx: mpsc::Sender<FeedItem>,
    /// Proactive engine for background monitoring.
    pub proactive: proactive::ProactiveEngine,
    /// Background task manager.
    pub task_manager: tasks::TaskManager,
    /// Session context for proactive intelligence.
    pub session: context::SessionContext,
}

impl App {
    fn new() -> Self {
        let (brain_tx, brain_rx) = mpsc::channel();
        let (proactive_tx, proactive_rx) = mpsc::channel();

        let mut feed = FeedStore::new(200);

        // Welcome card
        let welcome = FeedItem::new(
            FeedSource::System,
            Priority::Normal,
            "Welcome to AetherOS v0.3".to_string(),
        )
        .with_body(vec![
            "AI-Native Operating System".to_string(),
            "Type anything. The OS understands you.".to_string(),
            "Use !cmd for shell, &query for background, Tab to navigate.".to_string(),
        ]);
        feed.push(welcome);

        let proactive_engine = proactive::ProactiveEngine::new(proactive_tx.clone());

        let mut app = Self {
            input: String::new(),
            cursor: 0,
            feed,
            feed_scroll: 0,
            telemetry: telemetry::SysTelemetry::default(),
            aurora: aurora_client::AuroraStatus::default(),
            quit: false,
            history: Vec::new(),
            history_pos: None,
            thinking: false,
            thinking_frame: 0,
            brain_rx,
            brain_tx,
            active_panel: ActivePanel::Input,
            selected_feed_item: None,
            proactive_rx,
            proactive_tx,
            proactive: proactive_engine,
            task_manager: tasks::TaskManager::new(),
            session: context::SessionContext::load(),
        };
        app.telemetry = telemetry::read_telemetry();
        app.aurora = aurora_client::check_health();

        // Initial system health card
        app.push_system_health_card();

        app
    }

    fn push_system_health_card(&mut self) {
        let t = &self.telemetry;
        let mem_used = t.mem_total_mb.saturating_sub(t.mem_avail_mb);
        let mem_pct = if t.mem_total_mb > 0 {
            (mem_used as f64 / t.mem_total_mb as f64) * 100.0
        } else {
            0.0
        };

        let card = FeedItem::new(
            FeedSource::System,
            Priority::Low,
            "System Health".to_string(),
        )
        .with_body(vec![
            format!(
                "CPU: {:.0}% | Mem: {:.0}% ({}/{}MB) | Procs: {}",
                t.cpu_percent, mem_pct, mem_used, t.mem_total_mb, t.num_procs
            ),
            format!(
                "Kernel: {} | Cores: {} | Net: {}",
                t.kernel, t.cores, t.ip_addr
            ),
        ])
        .with_stale(120)
        .with_replaces(FeedSource::System);

        self.feed.push(card);
    }

    fn push_brain_response(&mut self, resp: brain_client::BrainResponse) {
        let mut body: Vec<String> = Vec::new();
        if !resp.text.is_empty() {
            for line in resp.text.lines() {
                body.push(line.to_string());
            }
        }

        let mut card = FeedItem::new(
            FeedSource::Brain,
            Priority::Normal,
            "Brain Response".to_string(),
        )
        .with_body(body);

        // Add first widget to the main card
        if let Some(first_widget) = resp.widgets.first() {
            let color = widget_color(&first_widget.widget_type);
            card = card.with_widget(WidgetData {
                widget_type: first_widget.widget_type.clone(),
                title: first_widget.title.clone(),
                lines: first_widget.lines.clone(),
                color,
            });
        }

        // Additional widgets as separate cards
        for widget in resp.widgets.iter().skip(1) {
            let color = widget_color(&widget.widget_type);
            let widget_card = FeedItem::new(
                FeedSource::Brain,
                Priority::Normal,
                widget.title.clone(),
            )
            .with_widget(WidgetData {
                widget_type: widget.widget_type.clone(),
                title: widget.title.clone(),
                lines: widget.lines.clone(),
                color,
            });
            self.feed.push(widget_card);
        }

        if resp.latency_ms > 0 {
            card.body
                .push(format!("[{:.1}s]", resp.latency_ms as f64 / 1000.0));
        }

        self.feed.push(card);
    }

    fn submit_command(&mut self) {
        let cmd = self.input.trim().to_string();
        if cmd.is_empty() {
            return;
        }

        // Echo user input as a card
        let user_card = FeedItem::new(FeedSource::User, Priority::Normal, format!("> {}", cmd));
        self.feed.push(user_card);

        self.history.push(cmd.clone());
        self.history_pos = None;
        self.input.clear();
        self.cursor = 0;
        self.feed_scroll = 0;

        // Local-only commands
        let lower = cmd.to_lowercase();
        match lower.as_str() {
            "help" => {
                let result = commands::help_text();
                let card =
                    FeedItem::new(FeedSource::System, Priority::Normal, "Help".to_string())
                        .with_body(result.lines().map(|l| l.to_string()).collect());
                self.feed.push(card);
                return;
            }
            "clear" => {
                self.feed.clear();
                return;
            }
            "exit" | "quit" => {
                self.quit = true;
                return;
            }
            "sysinfo" => {
                self.telemetry = telemetry::read_telemetry();
                let result = commands::sysinfo_text(&self.telemetry);
                let card = FeedItem::new(
                    FeedSource::System,
                    Priority::Normal,
                    "System Info".to_string(),
                )
                .with_body(result.lines().map(|l| l.to_string()).collect());
                self.feed.push(card);
                return;
            }
            "tasks" => {
                let summary = self.task_manager.summary();
                let active = self.task_manager.active_tasks();
                let mut body = vec![summary];
                for (name, elapsed) in active {
                    body.push(format!("  {} ({}s)", name, elapsed));
                }
                let card = FeedItem::new(
                    FeedSource::System,
                    Priority::Normal,
                    "Background Tasks".to_string(),
                )
                .with_body(body);
                self.feed.push(card);
                return;
            }
            _ => {}
        }

        // Background task with & prefix
        if cmd.starts_with('&') {
            let query = cmd[1..].trim();
            if query.starts_with('!') {
                // Background shell: &!ls -la
                let shell_cmd = &query[1..];
                match self.task_manager.spawn_shell_task(shell_cmd) {
                    Some(_) => {
                        let card = FeedItem::new(
                            FeedSource::Task,
                            Priority::Low,
                            format!("Queued: !{}", shell_cmd),
                        );
                        self.feed.push(card);
                    }
                    None => {
                        let card = FeedItem::new(
                            FeedSource::System,
                            Priority::Normal,
                            "Too many tasks".to_string(),
                        )
                        .with_body(vec!["Maximum 10 concurrent background tasks.".to_string()]);
                        self.feed.push(card);
                    }
                }
            } else {
                // Background brain query: &weather in Tokyo
                match self.task_manager.spawn_brain_task(query) {
                    Some(_) => {
                        let card = FeedItem::new(
                            FeedSource::Task,
                            Priority::Low,
                            format!("Queued: {}", query),
                        );
                        self.feed.push(card);
                    }
                    None => {
                        let card = FeedItem::new(
                            FeedSource::System,
                            Priority::Normal,
                            "Too many tasks".to_string(),
                        )
                        .with_body(vec!["Maximum 10 concurrent background tasks.".to_string()]);
                        self.feed.push(card);
                    }
                }
            }
            return;
        }

        // Shell passthrough with ! prefix
        if cmd.starts_with('!') {
            let shell_cmd = &cmd[1..];
            let result = commands::run_shell(shell_cmd);
            let card = FeedItem::new(
                FeedSource::Task,
                Priority::Normal,
                format!("Shell: {}", shell_cmd),
            )
            .with_body(result.lines().map(|l| l.to_string()).collect());
            self.feed.push(card);
            return;
        }

        // Everything else goes to brain (async, blocking input)
        self.session.record_query(&cmd);
        self.proactive.set_last_query(&cmd);
        self.thinking = true;
        self.thinking_frame = 0;
        let tx = self.brain_tx.clone();
        let input_str = cmd;
        std::thread::spawn(move || {
            let result = match brain_client::query_brain(&input_str) {
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

    /// Cycle to the next panel.
    fn cycle_panel(&mut self) {
        self.active_panel = match self.active_panel {
            ActivePanel::Input => ActivePanel::Feed,
            ActivePanel::Feed => ActivePanel::Sidebar,
            ActivePanel::Sidebar => ActivePanel::Input,
        };
        if self.active_panel == ActivePanel::Feed {
            let count = self.feed.visible_items().len();
            self.selected_feed_item = if count > 0 { Some(count - 1) } else { None };
        }
    }

    fn feed_select_prev(&mut self) {
        if let Some(idx) = self.selected_feed_item {
            if idx > 0 {
                self.selected_feed_item = Some(idx - 1);
            }
        }
    }

    fn feed_select_next(&mut self) {
        if let Some(idx) = self.selected_feed_item {
            let max = self.feed.visible_items().len().saturating_sub(1);
            if idx < max {
                self.selected_feed_item = Some(idx + 1);
            }
        }
    }

    fn feed_toggle_collapse(&mut self) {
        if let Some(idx) = self.selected_feed_item {
            let visible = self.feed.visible_items();
            if let Some(item) = visible.get(idx) {
                let id = item.id;
                self.feed.toggle_collapse(id);
            }
        }
    }

    fn feed_dismiss(&mut self) {
        if let Some(idx) = self.selected_feed_item {
            let visible = self.feed.visible_items();
            if let Some(item) = visible.get(idx) {
                let id = item.id;
                self.feed.dismiss(id);
                let new_count = self.feed.visible_items().len();
                if new_count == 0 {
                    self.selected_feed_item = None;
                } else if idx >= new_count {
                    self.selected_feed_item = Some(new_count - 1);
                }
            }
        }
    }

    /// Handle an action from the input router.
    fn handle_action(&mut self, action: AppAction) {
        match action {
            AppAction::Quit => self.quit = true,
            AppAction::SwitchPanel => self.cycle_panel(),
            AppAction::ReturnToInput => self.active_panel = ActivePanel::Input,

            AppAction::TypeChar(c) => {
                self.input.insert(self.cursor, c);
                self.cursor += 1;
            }
            AppAction::Backspace => {
                if self.cursor > 0 {
                    self.cursor -= 1;
                    self.input.remove(self.cursor);
                }
            }
            AppAction::Delete => {
                if self.cursor < self.input.len() {
                    self.input.remove(self.cursor);
                }
            }
            AppAction::Submit => self.submit_command(),
            AppAction::CursorLeft => {
                if self.cursor > 0 {
                    self.cursor -= 1;
                }
            }
            AppAction::CursorRight => {
                if self.cursor < self.input.len() {
                    self.cursor += 1;
                }
            }
            AppAction::CursorHome => self.cursor = 0,
            AppAction::CursorEnd => self.cursor = self.input.len(),
            AppAction::HistoryUp => {
                if !self.history.is_empty() {
                    let pos = match self.history_pos {
                        Some(p) if p > 0 => p - 1,
                        Some(p) => p,
                        None => self.history.len() - 1,
                    };
                    self.history_pos = Some(pos);
                    self.input = self.history[pos].clone();
                    self.cursor = self.input.len();
                }
            }
            AppAction::HistoryDown => {
                if let Some(pos) = self.history_pos {
                    if pos + 1 < self.history.len() {
                        let new_pos = pos + 1;
                        self.history_pos = Some(new_pos);
                        self.input = self.history[new_pos].clone();
                        self.cursor = self.input.len();
                    } else {
                        self.history_pos = None;
                        self.input.clear();
                        self.cursor = 0;
                    }
                }
            }

            AppAction::FeedSelectPrev => self.feed_select_prev(),
            AppAction::FeedSelectNext => self.feed_select_next(),
            AppAction::FeedToggleCollapse => self.feed_toggle_collapse(),
            AppAction::FeedDismiss => self.feed_dismiss(),
            AppAction::FeedPageUp => {
                self.feed_scroll = self.feed_scroll.saturating_add(10);
            }
            AppAction::FeedPageDown => {
                self.feed_scroll = self.feed_scroll.saturating_sub(10);
            }

            AppAction::PageUp => {
                self.feed_scroll = self.feed_scroll.saturating_add(10);
            }
            AppAction::PageDown => {
                self.feed_scroll = self.feed_scroll.saturating_sub(10);
            }

            AppAction::TriggerSysinfo => {
                self.telemetry = telemetry::read_telemetry();
                let result = commands::sysinfo_text(&self.telemetry);
                let card = FeedItem::new(
                    FeedSource::System,
                    Priority::Normal,
                    "System Info".to_string(),
                )
                .with_body(result.lines().map(|l| l.to_string()).collect());
                self.feed.push(card);
            }
            AppAction::TriggerWorldModel => {
                match aurora_client::query_introspect() {
                    Ok(data) => {
                        let card = FeedItem::new(
                            FeedSource::WorldModel,
                            Priority::Normal,
                            "World Model Status".to_string(),
                        )
                        .with_body(vec![
                            format!(
                                "Weights: {} | Params: {}M",
                                data.weight_version,
                                data.param_count / 1_000_000
                            ),
                            format!(
                                "Predictions: {} | Avg latency: {:.0}ms",
                                data.total_predictions, data.mean_latency_ms
                            ),
                            format!(
                                "Learning: {} | Updates: {} | Avg error: {:.3}",
                                if data.learning_enabled {
                                    "enabled"
                                } else {
                                    "disabled"
                                },
                                data.total_updates,
                                data.mean_prediction_error
                            ),
                        ]);
                        self.feed.push(card);
                    }
                    Err(e) => {
                        let card = FeedItem::new(
                            FeedSource::WorldModel,
                            Priority::Normal,
                            "World Model Unavailable".to_string(),
                        )
                        .with_body(vec![format!("Could not reach CFC-JEPA: {}", e)]);
                        self.feed.push(card);
                    }
                }
            }

            AppAction::Noop => {}
        }
    }
}

fn widget_color(widget_type: &str) -> ui::BlockColor {
    match widget_type {
        "weather" => ui::BlockColor::Yellow,
        "system" => ui::BlockColor::Green,
        "file" => ui::BlockColor::Blue,
        "table" => ui::BlockColor::Cyan,
        _ => ui::BlockColor::White,
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
    let tick_rate = Duration::from_millis(250);
    let mut last_tick = Instant::now();
    let mut telemetry_interval = Instant::now();
    let mut health_card_interval = Instant::now();

    loop {
        // Render
        terminal.draw(|f| ui::draw(f, &app))?;

        // Input
        let timeout = tick_rate.saturating_sub(last_tick.elapsed());
        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                let action = input::route(key, &app.active_panel, app.thinking);
                app.handle_action(action);
            }
        }

        // Brain response check
        if app.thinking {
            if let Ok(resp) = app.brain_rx.try_recv() {
                app.thinking = false;
                app.push_brain_response(resp);
            } else {
                app.thinking_frame = app.thinking_frame.wrapping_add(1);
            }
        }

        // Proactive feed items
        while let Ok(item) = app.proactive_rx.try_recv() {
            app.feed.push(item);
        }

        // Background task completions
        let task_items = app.task_manager.tick();
        for item in task_items {
            app.feed.push(item);
        }

        // Periodic tick
        if last_tick.elapsed() >= tick_rate {
            last_tick = Instant::now();
        }

        // Telemetry refresh every 2 seconds
        if telemetry_interval.elapsed() >= Duration::from_secs(2) {
            app.telemetry = telemetry::read_telemetry();
            // Feed task + session context into proactive engine
            let (active, completed) = app.task_manager.counts();
            app.proactive.set_task_counts(active, completed);
            app.proactive.set_user_topics(app.session.top_topics(5));
            app.proactive.tick(&app.telemetry);
            telemetry_interval = Instant::now();
        }

        // System health card every 30 seconds
        if health_card_interval.elapsed() >= Duration::from_secs(30) {
            app.push_system_health_card();
            health_card_interval = Instant::now();
        }

        // Prune stale feed items
        app.feed.prune_stale();

        // Periodic session context save
        app.session.maybe_save();

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
