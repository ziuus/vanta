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

/// 3x5 block-digit font. Each glyph is 5 rows; digits are 3 wide, colon is 1.
fn glyph(c: char) -> [&'static str; 5] {
    match c {
        '0' => ["███", "█ █", "█ █", "█ █", "███"],
        '1' => ["  █", "  █", "  █", "  █", "  █"],
        '2' => ["███", "  █", "███", "█  ", "███"],
        '3' => ["███", "  █", "███", "  █", "███"],
        '4' => ["█ █", "█ █", "███", "  █", "  █"],
        '5' => ["███", "█  ", "███", "  █", "███"],
        '6' => ["███", "█  ", "███", "█ █", "███"],
        '7' => ["███", "  █", "  █", "  █", "  █"],
        '8' => ["███", "█ █", "███", "█ █", "███"],
        '9' => ["███", "█ █", "███", "  █", "███"],
        ':' => [" ", "█", " ", "█", " "],
        _ => ["   ", "   ", "   ", "   ", "   "],
    }
}

/// Build the 5 rows of block art for a time string like "09:33:52".
fn block_time(s: &str) -> Vec<String> {
    let glyphs: Vec<[&'static str; 5]> = s.chars().map(glyph).collect();
    (0..5)
        .map(|row| {
            glyphs
                .iter()
                .map(|g| g[row])
                .collect::<Vec<_>>()
                .join(" ")
        })
        .collect()
}

pub fn render(f: &mut Frame, area: Rect, theme: &app::Theme) {
    let now = Local::now();

    let weekday = now.format("%a").to_string();
    let month = now.format("%b").to_string();
    let day = now.format("%d").to_string();
    let year = now.format("%Y").to_string();
    let date_str = format!("{} {} {}, {}", weekday, month, day, year);
    let uptime = format_uptime();
    let dt = format!("{}  ·  Up {}", date_str, uptime);

    if area.height < 2 {
        return;
    }

    // Big block clock when there's room; else fall back to plain one-liner.
    if area.width >= 34 && area.height >= 6 {
        let time_str = now.format("%H:%M:%S").to_string();
        let art = block_time(&time_str);

        let mut lines: Vec<Line> = art
            .into_iter()
            .map(|row| {
                Line::from(Span::styled(
                    row,
                    Style::default()
                        .fg(theme.accent)
                        .add_modifier(ratatui::style::Modifier::BOLD),
                ))
            })
            .collect();
        lines.push(Line::from(Span::styled(dt, Style::default().fg(theme.dim))));

        let top = (area.height.saturating_sub(lines.len() as u16)) / 2;
        let inner = Rect::new(area.x, area.y + top, area.width, area.height - top);
        f.render_widget(
            Paragraph::new(lines).alignment(ratatui::layout::Alignment::Center),
            inner,
        );
        return;
    }

    // Compact fallback: single time line + date.
    let time_str = if area.width >= 10 {
        now.format("%H:%M:%S").to_string()
    } else {
        now.format("%H:%M").to_string()
    };
    let top = (area.height.saturating_sub(2)) / 2;
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
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(dt, Style::default().fg(theme.dim))))
            .alignment(ratatui::layout::Alignment::Center),
        Rect::new(area.x, area.y + top + 1, area.width, area.height - top - 1),
    );
}
