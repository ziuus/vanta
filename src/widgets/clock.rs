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

/// Heavy 7x6 digit font (peaclock-weight): 2-cell-thick strokes so the glyphs
/// read as solid slabs rather than thin outlines.
fn big_glyph(c: char) -> [&'static str; 7] {
    match c {
        '0' => ["██████", "██  ██", "██  ██", "██  ██", "██  ██", "██  ██", "██████"],
        '1' => ["    ██", "    ██", "    ██", "    ██", "    ██", "    ██", "    ██"],
        '2' => ["██████", "    ██", "    ██", "██████", "██    ", "██    ", "██████"],
        '3' => ["██████", "    ██", "    ██", "██████", "    ██", "    ██", "██████"],
        '4' => ["██  ██", "██  ██", "██  ██", "██████", "    ██", "    ██", "    ██"],
        '5' => ["██████", "██    ", "██    ", "██████", "    ██", "    ██", "██████"],
        '6' => ["██████", "██    ", "██    ", "██████", "██  ██", "██  ██", "██████"],
        '7' => ["██████", "    ██", "    ██", "    ██", "    ██", "    ██", "    ██"],
        '8' => ["██████", "██  ██", "██  ██", "██████", "██  ██", "██  ██", "██████"],
        '9' => ["██████", "██  ██", "██  ██", "██████", "    ██", "    ██", "██████"],
        ':' => ["  ", "██", "██", "  ", "██", "██", "  "],
        _ => ["      ", "      ", "      ", "      ", "      ", "      ", "      "],
    }
}

/// 3x5 block-digit font — the mid-size fallback for narrower panels.
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

/// Join per-glyph rows into full lines, one space between glyphs.
fn compose<const N: usize>(s: &str, f: impl Fn(char) -> [&'static str; N]) -> Vec<String> {
    let glyphs: Vec<[&'static str; N]> = s.chars().map(f).collect();
    (0..N)
        .map(|row| {
            glyphs
                .iter()
                .map(|g| g[row])
                .collect::<Vec<_>>()
                .join(" ")
        })
        .collect()
}

fn styled(rows: Vec<String>, theme: &app::Theme) -> Vec<Line<'static>> {
    rows.into_iter()
        .map(|r| {
            Line::from(Span::styled(
                r,
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(ratatui::style::Modifier::BOLD),
            ))
        })
        .collect()
}

pub fn render(f: &mut Frame, area: Rect, theme: &app::Theme) {
    let now = Local::now();

    let date_str = now.format("%a %b %d, %Y").to_string();
    let dt = format!("{}  ·  Up {}", date_str, format_uptime());

    if area.height < 2 {
        return;
    }

    // Pick the heaviest font the panel can hold: 7-row slabs, then 5-row
    // blocks, then a plain digital readout.
    let time_str = now.format("%H:%M:%S").to_string();
    let mut lines: Vec<Line<'static>> = if area.width >= 48 && area.height >= 7 {
        styled(compose(&time_str, big_glyph), theme)
    } else if area.width >= 34 && area.height >= 6 {
        styled(compose(&time_str, glyph), theme)
    } else {
        let short = if area.width >= 10 {
            time_str
        } else {
            now.format("%H:%M").to_string()
        };
        vec![Line::from(Span::styled(
            short,
            Style::default()
                .fg(theme.accent)
                .add_modifier(ratatui::style::Modifier::BOLD),
        ))]
    };

    // Blank spacer then the date/uptime caption, when there's room.
    if area.height as usize > lines.len() + 1 {
        lines.push(Line::from(""));
    }
    if area.height as usize > lines.len() {
        lines.push(Line::from(Span::styled(dt, Style::default().fg(theme.dim))));
    }

    let top = (area.height.saturating_sub(lines.len() as u16)) / 2;
    f.render_widget(
        Paragraph::new(lines).alignment(ratatui::layout::Alignment::Center),
        Rect::new(area.x, area.y + top, area.width, area.height - top),
    );
}
