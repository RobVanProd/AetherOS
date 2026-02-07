use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Paragraph, Wrap},
};

use crate::{App, OutputBlock};

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
}

impl BlockColor {
    fn to_color(&self) -> Color {
        match self {
            BlockColor::Cyan => Color::Cyan,
            BlockColor::Green => Color::Green,
            BlockColor::Yellow => Color::Yellow,
            BlockColor::Blue => Color::LightBlue,
            BlockColor::Red => Color::Red,
            BlockColor::White => Color::White,
            BlockColor::DarkGray => Color::DarkGray,
        }
    }
}

pub fn draw(f: &mut Frame, app: &App) {
    let size = f.area();

    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // compact header
            Constraint::Min(10),  // output (full width)
            Constraint::Length(3), // input bar
        ])
        .split(size);

    draw_header(f, main_chunks[0], app);
    draw_output(f, main_chunks[1], app);
    draw_input(f, main_chunks[2], app);
}

fn draw_header(f: &mut Frame, area: Rect, app: &App) {
    let uptime = app.telemetry.uptime_secs;
    let up_str = if uptime >= 3600 {
        format!("{}h{}m", uptime / 3600, (uptime % 3600) / 60)
    } else if uptime >= 60 {
        format!("{}m{}s", uptime / 60, uptime % 60)
    } else {
        format!("{}s", uptime)
    };

    let brain_status = if app.thinking {
        Span::styled("thinking", Style::default().fg(Color::Yellow).bold())
    } else {
        Span::styled("ready", Style::default().fg(Color::Green))
    };

    let header = Paragraph::new(Line::from(vec![
        Span::styled(
            " AETHER OS ",
            Style::default().fg(Color::Black).bg(Color::Cyan).bold(),
        ),
        Span::raw(" Brain: "),
        brain_status,
        Span::styled(
            format!(" | Up: {} | CPU: {:.0}% | Mem: {}MB",
                up_str,
                app.telemetry.cpu_percent,
                app.telemetry.mem_total_mb.saturating_sub(app.telemetry.mem_avail_mb)),
            Style::default().fg(Color::DarkGray),
        ),
    ]));
    f.render_widget(header, area);
}

fn draw_output(f: &mut Frame, area: Rect, app: &App) {
    let inner_height = area.height.saturating_sub(2) as usize;
    let inner_width = area.width.saturating_sub(2) as usize;

    // Convert OutputBlocks to Lines for rendering
    let mut all_lines: Vec<Line> = Vec::new();

    for block in &app.output {
        match block {
            OutputBlock::Text(text) => {
                all_lines.push(Line::from(Span::raw(text.as_str())));
            }
            OutputBlock::Styled { text, color } => {
                all_lines.push(Line::from(Span::styled(
                    text.as_str(),
                    Style::default().fg(color.to_color()).bold(),
                )));
            }
            OutputBlock::Widget { title, lines, color } => {
                let c = color.to_color();
                let box_width = inner_width.saturating_sub(2).min(60);
                let top = format!("\u{250c}\u{2500}\u{2500}\u{2500} {} {}\u{2510}",
                    title,
                    "\u{2500}".repeat(box_width.saturating_sub(title.len() + 6)));
                all_lines.push(Line::from(Span::styled(top, Style::default().fg(c))));

                for line in lines {
                    let padded = if line.len() < box_width - 2 {
                        format!("\u{2502} {}{} \u{2502}",
                            line,
                            " ".repeat(box_width.saturating_sub(line.len() + 4)))
                    } else {
                        format!("\u{2502} {} \u{2502}", &line[..box_width.saturating_sub(4).min(line.len())])
                    };
                    all_lines.push(Line::from(Span::styled(padded, Style::default().fg(c))));
                }

                let bottom = format!("\u{2514}{}\u{2518}",
                    "\u{2500}".repeat(box_width.saturating_sub(2)));
                all_lines.push(Line::from(Span::styled(bottom, Style::default().fg(c))));
            }
            OutputBlock::Separator => {
                all_lines.push(Line::from(""));
            }
        }
    }

    // Add thinking indicator
    if app.thinking {
        let dots = match app.thinking_frame % 4 {
            0 => ".",
            1 => "..",
            2 => "...",
            _ => "",
        };
        all_lines.push(Line::from(Span::styled(
            format!("  Thinking{}", dots),
            Style::default().fg(Color::Yellow).bold(),
        )));
    }

    // Scrolling
    let total = all_lines.len();
    let scroll = app.scroll as usize;
    let end = total.saturating_sub(scroll);
    let start = end.saturating_sub(inner_height);
    let visible: Vec<Line> = all_lines[start..end].to_vec();

    let output = Paragraph::new(visible)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray)),
        )
        .wrap(Wrap { trim: false });

    f.render_widget(output, area);
}

fn draw_input(f: &mut Frame, area: Rect, app: &App) {
    let prompt = if app.thinking {
        Span::styled("> ", Style::default().fg(Color::Yellow).bold())
    } else {
        Span::styled("> ", Style::default().fg(Color::Cyan).bold())
    };

    let input = Paragraph::new(Line::from(vec![
        prompt,
        Span::raw(&app.input),
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(if app.thinking {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default().fg(Color::Cyan)
            }),
    );
    f.render_widget(input, area);

    // Position cursor
    if !app.thinking {
        f.set_cursor_position(Position::new(
            area.x + 3 + app.cursor as u16,
            area.y + 1,
        ));
    }
}
