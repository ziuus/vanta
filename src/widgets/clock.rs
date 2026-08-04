use std::fs;

use chrono::Local;
use ratatui::layout::Rect;
use ratatui::style::Style;
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
    let now = Local::now();

    let weekday = now.format("%a").to_string();
    let month = now.format("%b").to_string();
    let day = now.format("%d").to_string();
    let year = now.format("%Y").to_string();
    let time_str = if area.width >= 10 {
        now.format("%H:%M:%S").to_string()
    } else {
        now.format("%H:%M").to_string()
    };
    let date_str = format!("{} {} {}, {}", weekday, month, day, year);
    let uptime = format_uptime();

    if area.height < 2 {
        return;
    }

    // Vertically center the two content lines when the panel is taller.
    let top = (area.height.saturating_sub(2)) / 2;

    // Time first (always), then date+uptime below when there's room.
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            time_str,
            Style::default()
                .fg(theme.accent)
                .add_modifier(ratatui::style::Modifier::BOLD),
        )))
        .alignment(ratatui::layout::Alignment::Center),
        Rect::new(area.x, area.y + top, area.width, 1),
    );

    let dt = format!("{}  ·  Up {}", date_str, uptime);
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(dt, Style::default().fg(theme.dim))))
            .alignment(ratatui::layout::Alignment::Center),
        Rect::new(area.x, area.y + top + 1, area.width, area.height - top - 1),
    );
}
