use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Paragraph, Wrap},
};

use crate::App;

pub fn draw(f: &mut Frame, app: &App) {
    let size = f.area();

    // Main layout: header, body, input bar
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // header
            Constraint::Min(10),   // body
            Constraint::Length(3), // input bar
        ])
        .split(size);

    draw_header(f, main_chunks[0]);
    draw_body(f, main_chunks[1], app);
    draw_input(f, main_chunks[2], app);
}

fn draw_header(f: &mut Frame, area: Rect) {
    let header = Paragraph::new(Line::from(vec![
        Span::styled(
            " AETHER OS ",
            Style::default().fg(Color::Black).bg(Color::Cyan).bold(),
        ),
        Span::raw("  "),
        Span::styled(
            "v0.3.0",
            Style::default().fg(Color::DarkGray),
        ),
        Span::raw("  "),
        Span::styled(
            "Self-Modifying AI OS",
            Style::default().fg(Color::Cyan),
        ),
        Span::raw("  |  "),
        Span::styled(
            "Aeternum Labs",
            Style::default().fg(Color::Yellow),
        ),
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray)),
    );
    f.render_widget(header, area);
}

fn draw_body(f: &mut Frame, area: Rect, app: &App) {
    // Body: [left panels] | [output]
    let body_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(30), // side panels
            Constraint::Min(30),   // output
        ])
        .split(area);

    // Side panels: system + aurora
    let side_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(50),
            Constraint::Percentage(50),
        ])
        .split(body_chunks[0]);

    draw_system_panel(f, side_chunks[0], app);
    draw_aurora_panel(f, side_chunks[1], app);
    draw_output(f, body_chunks[1], app);
}

fn draw_system_panel(f: &mut Frame, area: Rect, app: &App) {
    let t = &app.telemetry;

    let cpu_label = format!("CPU: {:5.1}%", t.cpu_percent);
    let mem_used = t.mem_total_mb.saturating_sub(t.mem_avail_mb);
    let mem_label = format!("MEM: {}/{}MB", mem_used, t.mem_total_mb);
    let mem_ratio = if t.mem_total_mb > 0 {
        mem_used as f64 / t.mem_total_mb as f64
    } else {
        0.0
    };

    let mut lines = vec![
        Line::from(Span::styled(&cpu_label, Style::default().fg(Color::Green))),
    ];

    // CPU bar
    let cpu_bar_width = (area.width as usize).saturating_sub(4).min(24);
    let filled = ((t.cpu_percent / 100.0) * cpu_bar_width as f64) as usize;
    let bar: String = format!(
        " [{}{}]",
        "#".repeat(filled.min(cpu_bar_width)),
        " ".repeat(cpu_bar_width.saturating_sub(filled))
    );
    lines.push(Line::from(Span::styled(bar, Style::default().fg(Color::Green))));

    lines.push(Line::from(Span::styled(&mem_label, Style::default().fg(Color::Blue))));
    let mem_filled = (mem_ratio * cpu_bar_width as f64) as usize;
    let mem_bar: String = format!(
        " [{}{}]",
        "#".repeat(mem_filled.min(cpu_bar_width)),
        " ".repeat(cpu_bar_width.saturating_sub(mem_filled))
    );
    lines.push(Line::from(Span::styled(mem_bar, Style::default().fg(Color::Blue))));

    lines.push(Line::from(""));
    lines.push(Line::from(Span::raw(format!("Net:  {}", t.ip_addr))));
    lines.push(Line::from(Span::raw(format!("Up:   {}s", t.uptime_secs))));
    lines.push(Line::from(Span::raw(format!("Procs: {}", t.num_procs))));

    let panel = Paragraph::new(lines).block(
        Block::default()
            .title(" System ")
            .title_style(Style::default().fg(Color::Cyan).bold())
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray)),
    );
    f.render_widget(panel, area);
}

fn draw_aurora_panel(f: &mut Frame, area: Rect, app: &App) {
    let a = &app.aurora;

    let status_color = if a.connected { Color::Green } else { Color::Red };
    let status_text = if a.connected { "online" } else { "offline" };

    let mut lines = vec![
        Line::from(vec![
            Span::raw("Status: "),
            Span::styled(status_text, Style::default().fg(status_color).bold()),
        ]),
    ];

    if a.connected {
        lines.push(Line::from(Span::raw(format!("Model:  CFC-JEPA 37M"))));
        lines.push(Line::from(Span::raw(format!("Preds:  {}", a.predictions))));
        if !a.weight_version.is_empty() {
            lines.push(Line::from(Span::raw(format!("Weight: {}", a.weight_version))));
        }
        let learn_str = if a.learning_enabled { "enabled" } else { "disabled" };
        let learn_color = if a.learning_enabled { Color::Green } else { Color::Yellow };
        lines.push(Line::from(vec![
            Span::raw("Learn:  "),
            Span::styled(learn_str, Style::default().fg(learn_color)),
        ]));
        if a.error > 0.0 {
            lines.push(Line::from(Span::raw(format!("Error:  {:.4}", a.error))));
        }
        if !a.gate_stats.is_empty() {
            lines.push(Line::from(""));
            for gs in &a.gate_stats {
                lines.push(Line::from(Span::styled(
                    gs.clone(),
                    Style::default().fg(Color::DarkGray),
                )));
            }
        }
    } else {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Run with cfcd for AI",
            Style::default().fg(Color::DarkGray),
        )));
    }

    let panel = Paragraph::new(lines).block(
        Block::default()
            .title(" Aurora AI ")
            .title_style(Style::default().fg(Color::Yellow).bold())
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray)),
    );
    f.render_widget(panel, area);
}

fn draw_output(f: &mut Frame, area: Rect, app: &App) {
    let inner_height = area.height.saturating_sub(2) as usize; // borders
    let total = app.output.len();
    let scroll = app.scroll as usize;

    // Calculate visible range
    let end = total.saturating_sub(scroll);
    let start = end.saturating_sub(inner_height);

    let visible: Vec<Line> = app.output[start..end]
        .iter()
        .map(|line| {
            if line.starts_with("> ") {
                Line::from(Span::styled(line.as_str(), Style::default().fg(Color::Cyan).bold()))
            } else if line.contains("error") || line.contains("Error") {
                Line::from(Span::styled(line.as_str(), Style::default().fg(Color::Red)))
            } else {
                Line::from(Span::raw(line.as_str()))
            }
        })
        .collect();

    let output = Paragraph::new(visible)
        .block(
            Block::default()
                .title(" Output ")
                .title_style(Style::default().fg(Color::White).bold())
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray)),
        )
        .wrap(Wrap { trim: false });

    f.render_widget(output, area);
}

fn draw_input(f: &mut Frame, area: Rect, app: &App) {
    let input = Paragraph::new(Line::from(vec![
        Span::styled("> ", Style::default().fg(Color::Cyan).bold()),
        Span::raw(&app.input),
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan)),
    );
    f.render_widget(input, area);

    // Position cursor
    f.set_cursor_position(Position::new(
        area.x + 3 + app.cursor as u16,
        area.y + 1,
    ));
}
