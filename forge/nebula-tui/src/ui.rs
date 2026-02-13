use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Paragraph, Wrap},
};

use crate::feed::{FeedItem, Priority};
use crate::widgets;
use crate::App;

/// Color identifiers for styled output.
#[derive(Clone, Debug)]
pub enum BlockColor {
    Cyan,
    Green,
    Yellow,
    Blue,
    Red,
    White,
    DarkGray,
    Magenta,
}

impl BlockColor {
    pub fn to_color(&self) -> Color {
        match self {
            BlockColor::Cyan => Color::Cyan,
            BlockColor::Green => Color::Green,
            BlockColor::Yellow => Color::Yellow,
            BlockColor::Blue => Color::LightBlue,
            BlockColor::Red => Color::Red,
            BlockColor::White => Color::White,
            BlockColor::DarkGray => Color::DarkGray,
            BlockColor::Magenta => Color::Magenta,
        }
    }
}

/// Which panel has focus.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ActivePanel {
    Input,
    Feed,
    Sidebar,
}

pub fn draw(f: &mut Frame, app: &App) {
    let size = f.area();
    let show_sidebar = size.width >= 60;

    // Main vertical layout: status bar, body, input
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // status bar
            Constraint::Min(8),   // body (sidebar + feed)
            Constraint::Length(3), // input bar
        ])
        .split(size);

    draw_status_bar(f, main_chunks[0], app);

    if show_sidebar {
        let body_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(18), // sidebar
                Constraint::Min(30),   // feed
            ])
            .split(main_chunks[1]);

        draw_sidebar(f, body_chunks[0], app);
        draw_feed(f, body_chunks[1], app);
    } else {
        draw_feed(f, main_chunks[1], app);
    }

    draw_input(f, main_chunks[2], app);
}

fn draw_status_bar(f: &mut Frame, area: Rect, app: &App) {
    let uptime = app.telemetry.uptime_secs;
    let up_str = if uptime >= 3600 {
        format!("{}h{}m", uptime / 3600, (uptime % 3600) / 60)
    } else if uptime >= 60 {
        format!("{}m{}s", uptime / 60, uptime % 60)
    } else {
        format!("{}s", uptime)
    };

    let cpu = app.telemetry.cpu_percent;
    let mem_pct = if app.telemetry.mem_total_mb > 0 {
        let used = app.telemetry.mem_total_mb.saturating_sub(app.telemetry.mem_avail_mb);
        (used as f64 / app.telemetry.mem_total_mb as f64) * 100.0
    } else {
        0.0
    };

    let brain_status = if app.thinking {
        Span::styled(" thinking ", Style::default().fg(Color::Black).bg(Color::Yellow).bold())
    } else {
        Span::styled(" ready ", Style::default().fg(Color::Black).bg(Color::Green))
    };

    let net_indicator = if app.telemetry.ip_addr.starts_with("10.")
        || app.telemetry.ip_addr.starts_with("192.")
        || app.telemetry.ip_addr.starts_with("172.")
        || app.telemetry.ip_addr.contains("up")
    {
        Span::styled(" NET ", Style::default().fg(Color::Black).bg(Color::Green))
    } else {
        Span::styled(" NET ", Style::default().fg(Color::Black).bg(Color::Red))
    };

    let unseen = app.feed.unseen_count();
    let urgent = app.feed.unseen_urgent_count();
    let alert_span = if urgent > 0 {
        Span::styled(
            format!(" {} ", urgent),
            Style::default().fg(Color::Black).bg(Color::Red).bold(),
        )
    } else if unseen > 0 {
        Span::styled(
            format!(" {} ", unseen),
            Style::default().fg(Color::Black).bg(Color::Yellow),
        )
    } else {
        Span::raw("")
    };

    let header = Paragraph::new(Line::from(vec![
        Span::styled(
            " AETHER OS ",
            Style::default().fg(Color::Black).bg(Color::Cyan).bold(),
        ),
        Span::raw(" "),
        brain_status,
        Span::raw(" "),
        net_indicator,
        Span::styled(
            format!(" Up:{} CPU:{:.0}% Mem:{:.0}% ", up_str, cpu, mem_pct),
            Style::default().fg(Color::DarkGray),
        ),
        alert_span,
    ]));
    f.render_widget(header, area);
}

fn draw_sidebar(f: &mut Frame, area: Rect, app: &App) {
    let is_focused = app.active_panel == ActivePanel::Sidebar;
    let border_color = if is_focused { Color::Cyan } else { Color::DarkGray };

    let mut lines: Vec<Line> = Vec::new();

    // Uptime / clock
    let uptime = app.telemetry.uptime_secs;
    let up_str = if uptime >= 3600 {
        format!("{}h {}m", uptime / 3600, (uptime % 3600) / 60)
    } else if uptime >= 60 {
        format!("{}m {}s", uptime / 60, uptime % 60)
    } else {
        format!("{}s", uptime)
    };
    lines.push(Line::from(Span::styled(
        format!(" Up: {}", up_str),
        Style::default().fg(Color::White).bold(),
    )));
    lines.push(Line::from(""));

    // CPU bar + sparkline
    let cpu = app.telemetry.cpu_percent;
    let cpu_color = if cpu > 80.0 { Color::Red } else if cpu > 50.0 { Color::Yellow } else { Color::Green };
    let cpu_bar = widgets::mini_bar(cpu, 100.0, 10);
    lines.push(Line::from(vec![
        Span::styled(" CPU ", Style::default().fg(Color::White)),
        Span::styled(cpu_bar, Style::default().fg(cpu_color)),
    ]));
    let cpu_hist = app.proactive.cpu_history();
    if cpu_hist.len() > 2 {
        let spark = widgets::sparkline(&cpu_hist, 14);
        lines.push(Line::from(Span::styled(
            format!(" {}", spark),
            Style::default().fg(cpu_color),
        )));
    }
    lines.push(Line::from(""));

    // Memory bar + sparkline
    let mem_pct = if app.telemetry.mem_total_mb > 0 {
        let used = app.telemetry.mem_total_mb.saturating_sub(app.telemetry.mem_avail_mb);
        (used as f64 / app.telemetry.mem_total_mb as f64) * 100.0
    } else {
        0.0
    };
    let mem_color = if mem_pct > 85.0 { Color::Red } else if mem_pct > 60.0 { Color::Yellow } else { Color::Green };
    let mem_bar = widgets::mini_bar(mem_pct, 100.0, 10);
    lines.push(Line::from(vec![
        Span::styled(" Mem ", Style::default().fg(Color::White)),
        Span::styled(mem_bar, Style::default().fg(mem_color)),
    ]));
    let mem_hist = app.proactive.mem_pct_history();
    if mem_hist.len() > 2 {
        let spark = widgets::sparkline(&mem_hist, 14);
        lines.push(Line::from(Span::styled(
            format!(" {}", spark),
            Style::default().fg(mem_color),
        )));
    }
    lines.push(Line::from(""));

    // Network
    lines.push(Line::from(vec![
        Span::styled(" Net ", Style::default().fg(Color::White)),
        Span::styled(&app.telemetry.ip_addr, Style::default().fg(Color::DarkGray)),
    ]));
    lines.push(Line::from(""));

    // Processes
    lines.push(Line::from(Span::styled(
        format!(" Procs: {}", app.telemetry.num_procs),
        Style::default().fg(Color::DarkGray),
    )));
    lines.push(Line::from(""));

    // Tasks
    lines.push(Line::from(Span::styled(
        " Tasks",
        Style::default().fg(Color::White).bold(),
    )));
    let task_summary = app.task_manager.summary();
    lines.push(Line::from(Span::styled(
        format!("  {}", task_summary),
        Style::default().fg(Color::DarkGray),
    )));
    for (name, elapsed) in app.task_manager.active_tasks() {
        lines.push(Line::from(Span::styled(
            format!("  > {} {}s", name, elapsed),
            Style::default().fg(Color::Yellow),
        )));
    }

    // Navigation hint
    let remaining = area.height.saturating_sub(2) as usize;
    if lines.len() < remaining {
        for _ in lines.len()..remaining.saturating_sub(1) {
            lines.push(Line::from(""));
        }
        lines.push(Line::from(Span::styled(
            " Tab:switch",
            Style::default().fg(Color::DarkGray),
        )));
    }

    let sidebar = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(border_color))
                .title(Span::styled(" System ", Style::default().fg(Color::White).bold())),
        );
    f.render_widget(sidebar, area);
}

fn draw_feed(f: &mut Frame, area: Rect, app: &App) {
    let is_focused = app.active_panel == ActivePanel::Feed;
    let border_color = if is_focused { Color::Cyan } else { Color::DarkGray };

    let inner_height = area.height.saturating_sub(2) as usize;
    let inner_width = area.width.saturating_sub(2) as usize;

    let visible = app.feed.visible_items();
    let mut all_lines: Vec<Line> = Vec::new();

    for (idx, item) in visible.iter().enumerate() {
        let is_selected = app.active_panel == ActivePanel::Feed
            && app.selected_feed_item == Some(idx);

        render_feed_card(item, is_selected, inner_width, &mut all_lines);
    }

    // Thinking indicator
    if app.thinking {
        let dots = match app.thinking_frame % 4 {
            0 => ".",
            1 => "..",
            2 => "...",
            _ => "",
        };
        all_lines.push(Line::from(""));
        all_lines.push(Line::from(Span::styled(
            format!("  Thinking{}", dots),
            Style::default().fg(Color::Yellow).bold(),
        )));
    }

    // Empty state
    if all_lines.is_empty() && !app.thinking {
        all_lines.push(Line::from(""));
        all_lines.push(Line::from(Span::styled(
            "  No items yet. Type something below.",
            Style::default().fg(Color::DarkGray),
        )));
    }

    // Scrolling: show the most recent items (bottom-anchored)
    let total = all_lines.len();
    let scroll = app.feed_scroll as usize;
    let end = total.saturating_sub(scroll);
    let start = end.saturating_sub(inner_height);
    let visible_lines: Vec<Line> = if start < end {
        all_lines[start..end].to_vec()
    } else {
        Vec::new()
    };

    let feed = Paragraph::new(visible_lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(border_color))
                .title(Span::styled(" Feed ", Style::default().fg(Color::White).bold())),
        )
        .wrap(Wrap { trim: false });
    f.render_widget(feed, area);
}

/// Render a single feed item as a card into the line buffer.
fn render_feed_card(item: &FeedItem, selected: bool, max_width: usize, lines: &mut Vec<Line<'_>>) {
    let source_color = item.source.color().to_color();
    let border_color = match item.priority {
        Priority::Urgent => Color::Red,
        Priority::Normal => {
            if selected { Color::Cyan } else { Color::DarkGray }
        }
        Priority::Low => Color::DarkGray,
    };

    let select_indicator = if selected { ">" } else { " " };

    // Header line: [icon] Title                     age
    let age = item.age_str();
    let icon = item.source.icon();
    let title_max = max_width.saturating_sub(age.len() + 8);
    let title = if item.title.len() > title_max {
        format!("{}...", &item.title[..title_max.saturating_sub(3)])
    } else {
        item.title.clone()
    };
    let padding = max_width.saturating_sub(title.len() + age.len() + 7);

    lines.push(Line::from(vec![
        Span::styled(select_indicator, Style::default().fg(border_color)),
        Span::styled(
            format!("[{}]", icon),
            Style::default().fg(source_color).bold(),
        ),
        Span::raw(" "),
        Span::styled(
            title,
            if selected {
                Style::default().fg(Color::White).bold()
            } else if item.priority == Priority::Urgent {
                Style::default().fg(Color::Red).bold()
            } else {
                Style::default().fg(Color::White)
            },
        ),
        Span::raw(" ".repeat(padding.max(1))),
        Span::styled(age, Style::default().fg(Color::DarkGray)),
    ]));

    // Body lines (if not collapsed)
    if !item.collapsed {
        // Show body text
        for line in &item.body {
            let truncated = if line.len() > max_width.saturating_sub(4) {
                format!("{}...", &line[..max_width.saturating_sub(7)])
            } else {
                line.clone()
            };
            lines.push(Line::from(Span::styled(
                format!("  {}", truncated),
                Style::default().fg(Color::DarkGray),
            )));
        }

        // Show widget if present
        if let Some(ref widget) = item.widget {
            let wc = widget.color.to_color();
            let box_width = max_width.saturating_sub(4).min(56);

            let top = format!(
                "  \u{250c}\u{2500} {} {}\u{2510}",
                widget.title,
                "\u{2500}".repeat(box_width.saturating_sub(widget.title.len() + 5))
            );
            lines.push(Line::from(Span::styled(top, Style::default().fg(wc))));

            for wline in &widget.lines {
                let content = if wline.len() > box_width.saturating_sub(4) {
                    &wline[..box_width.saturating_sub(4)]
                } else {
                    wline.as_str()
                };
                let pad = box_width.saturating_sub(content.len() + 4);
                let row = format!("  \u{2502} {}{} \u{2502}", content, " ".repeat(pad));
                lines.push(Line::from(Span::styled(row, Style::default().fg(wc))));
            }

            let bottom = format!(
                "  \u{2514}{}\u{2518}",
                "\u{2500}".repeat(box_width.saturating_sub(2))
            );
            lines.push(Line::from(Span::styled(bottom, Style::default().fg(wc))));
        }
    } else if !item.body.is_empty() {
        // Collapsed: show first line as preview
        let preview = &item.body[0];
        let truncated = if preview.len() > max_width.saturating_sub(8) {
            format!("{}...", &preview[..max_width.saturating_sub(11)])
        } else {
            preview.clone()
        };
        lines.push(Line::from(Span::styled(
            format!("  {}", truncated),
            Style::default().fg(Color::DarkGray),
        )));
    }

    // Separator between cards
    lines.push(Line::from(""));
}

fn draw_input(f: &mut Frame, area: Rect, app: &App) {
    let is_focused = app.active_panel == ActivePanel::Input;
    let border_color = if app.thinking {
        Color::Yellow
    } else if is_focused {
        Color::Cyan
    } else {
        Color::DarkGray
    };

    let prompt = if app.thinking {
        Span::styled("> ", Style::default().fg(Color::Yellow).bold())
    } else {
        Span::styled("> ", Style::default().fg(Color::Cyan).bold())
    };

    let input = Paragraph::new(Line::from(vec![prompt, Span::raw(&app.input)]))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(border_color)),
        );
    f.render_widget(input, area);

    // Position cursor only when input panel is focused
    if is_focused && !app.thinking {
        f.set_cursor_position(Position::new(
            area.x + 3 + app.cursor as u16,
            area.y + 1,
        ));
    }
}
