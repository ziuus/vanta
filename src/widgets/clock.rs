use std::fs;

use chrono::Local;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::app;

fn format_uptime() -> String {
    let boot_time = fs::read_to_string("/proc/uptime")
        .ok()
        .and_then(|s| s.split_whitespace().next()?.parse::<f64>().ok())
        .unwrap_or(0.0);

    let total_secs = boot_time as u64;
    let days = total_secs / 86400;
    let hours = (total_secs % 86400) / 3600;
    let mins = (total_secs % 3600) / 60;

    if days > 0 {
        format!("{}d {}h {}m", days, hours, mins)
    } else if hours > 0 {
        format!("{}h {}m", hours, mins)
    } else {
        format!("{}m", mins)
    }
}

pub fn render(f: &mut Frame, area: Rect, theme: &app::Theme) {
    if area.height < 1 {
        return;
    }

    let now = Local::now();
    let time_str = now.format("%H:%M:%S").to_string();
    let date_str = now.format("%A, %B %d").to_string();
    let up_str = format!("up {}", format_uptime());

    // Two lines: time (big bold accent) + date · uptime (dim)
    // Vertically center both within the available area
    let content_h = 2u16;
    let top = area.y + area.height.saturating_sub(content_h) / 2;

    // Row 1 — time
    if top < area.y + area.height {
        f.render_widget(
            Paragraph::new(Line::from(Span::styled(
                time_str,
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            )))
            .alignment(Alignment::Center),
            Rect::new(area.x, top, area.width, 1),
        );
    }

    // Row 2 — date · uptime
    let row2 = top + 1;
    if row2 < area.y + area.height {
        let sub = format!("{} · {}", date_str, up_str);
        f.render_widget(
            Paragraph::new(Line::from(Span::styled(
                sub,
                Style::default().fg(theme.dim),
            )))
            .alignment(Alignment::Center),
            Rect::new(area.x, row2, area.width, 1),
        );
    }
}
