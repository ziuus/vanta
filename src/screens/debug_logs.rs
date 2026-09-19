use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::app::App;
use crate::logger;

pub fn render(f: &mut Frame, area: Rect, app: &mut App) {
    let t = &app.theme;
    let filter_text = if app.panel_states.log_target.is_empty() {
        "All Components".to_string()
    } else {
        format!("Component: {}", app.panel_states.log_target)
    };

    let b = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(t.dim))
        .title(Span::styled(
            format!(" Debug Logs (Hidden) - {} ", filter_text),
            Style::default().fg(t.accent),
        ))
        .title_bottom(Span::styled(
            " [~] Exit  [c] Clear  [s] Save  [t] Filter by Target ",
            Style::default().fg(t.dim),
        ));

    let logs = logger::LOGS.read().unwrap();

    // Filter by component if needed (app.panel_states.log_target could be used later)
    let filtered: Vec<_> = logs
        .iter()
        .filter(|l| {
            if app.panel_states.log_target.is_empty() {
                true
            } else {
                l.target.contains(&app.panel_states.log_target)
            }
        })
        .collect();

    let mut lines: Vec<Line> = filtered
        .iter()
        .rev()
        .take(area.height.saturating_sub(2) as usize)
        .rev()
        .map(|l| {
            let color = match l.level {
                log::Level::Error => Color::Red,
                log::Level::Warn => Color::Yellow,
                log::Level::Info => Color::Green,
                log::Level::Debug => Color::Blue,
                log::Level::Trace => Color::DarkGray,
            };
            Line::from(vec![
                Span::styled(format!("{} ", l.timestamp), Style::default().fg(t.dim)),
                Span::styled(format!("{:5} ", l.level), Style::default().fg(color)),
                Span::styled(format!("[{}] ", l.target), Style::default().fg(t.accent)),
                Span::styled(l.message.clone(), Style::default().fg(t.text)),
            ])
        })
        .collect();

    if lines.is_empty() {
        if app.panel_states.log_target.is_empty() {
            lines.push(Line::from(vec![Span::styled(
                "(No logs in ring buffer)",
                Style::default().fg(t.dim),
            )]));
        } else {
            lines.push(Line::from(vec![
                Span::styled(
                    format!(
                        "(No logs matching component filter \"{}\". Total logs in buffer: {} — press 't' then Enter to clear filter)",
                        app.panel_states.log_target,
                        logs.len()
                    ),
                    Style::default().fg(t.dim),
                )
            ]));
        }
    }

    if app.panel_states.log_target_input_active {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(3)].as_ref())
            .split(area);

        f.render_widget(Paragraph::new(lines).block(b), chunks[0]);

        let input_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(t.accent))
            .title(Span::styled(
                " Filter Target (Enter to apply) ",
                Style::default().fg(t.accent),
            ));

        f.render_widget(
            Paragraph::new(format!("{}_", app.panel_states.log_target)).block(input_block),
            chunks[1],
        );
    } else {
        f.render_widget(Paragraph::new(lines).block(b), area);
    }
}

pub fn save_logs(app: &mut App) {
    let logs = logger::LOGS.read().unwrap();
    let filtered: Vec<_> = logs
        .iter()
        .filter(|l| {
            if app.panel_states.log_target.is_empty() {
                true
            } else {
                l.target.contains(&app.panel_states.log_target)
            }
        })
        .collect();

    let mut out = String::new();
    for l in &filtered {
        out.push_str(&format!(
            "{} {:5} [{}] {}\n",
            l.timestamp, l.level, l.target, l.message
        ));
    }

    let target_name = if app.panel_states.log_target.is_empty() {
        "all".to_string()
    } else {
        app.panel_states
            .log_target
            .replace("::", "-")
            .replace("/", "_")
            .replace("\\", "_")
    };

    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let path = format!("{}/vanta-logs-{}.txt", home, target_name);
    if std::fs::write(&path, out).is_ok() {
        log::info!(target: "core", "Saved {} logs to {}", filtered.len(), path);
    }
}
