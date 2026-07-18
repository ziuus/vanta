use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::app;

#[derive(Debug)]
struct Battery {
    capacity: u8,
    status: String,
}

fn read_batteries() -> Vec<Battery> {
    let mut bats = Vec::new();
    let sys_dir = Path::new("/sys/class/power_supply");

    let entries = match fs::read_dir(sys_dir) {
        Ok(e) => e,
        Err(_) => return bats,
    };

    for entry in entries.flatten() {
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if !name_str.starts_with("BAT") {
            continue;
        }

        let base = entry.path();
        let capacity = fs::read_to_string(base.join("capacity"))
            .ok()
            .and_then(|s| s.trim().parse::<u8>().ok())
            .unwrap_or(0);
        let status = fs::read_to_string(base.join("status"))
            .ok()
            .map(|s| s.trim().to_string())
            .unwrap_or_default();
        bats.push(Battery { capacity, status });
    }

    bats
}

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

fn is_leap(year: i64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

fn big_char(c: char) -> [&'static str; 5] {
    match c {
        '0' => ["███", "█ █", "█ █", "█ █", "███"],
        '1' => [" ██", "  █", "  █", "  █", "███"],
        '2' => ["███", "  █", "███", "█  ", "███"],
        '3' => ["███", "  █", "███", "  █", "███"],
        '4' => ["█ █", "█ █", "███", "  █", "  █"],
        '5' => ["███", "█  ", "███", "  █", "███"],
        '6' => ["███", "█  ", "███", "█ █", "███"],
        '7' => ["███", "  █", "  █", "  █", "  █"],
        '8' => ["███", "█ █", "███", "█ █", "███"],
        '9' => ["███", "█ █", "███", "  █", "███"],
        ':' => ["   ", " █ ", "   ", " █ ", "   "],
        _   => ["   ", "   ", "   ", "   ", "   "],
    }
}

pub fn render(f: &mut Frame, area: Rect, theme: &app::Theme) {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();

    let secs = now.as_secs();
    let ist_offset = 5 * 3600 + 30 * 60;
    let local = (secs + ist_offset) as i64;
    let days = local / 86400;

    let weekday = match days.rem_euclid(7) {
        0 => "Thu", 1 => "Fri", 2 => "Sat", 3 => "Sun",
        4 => "Mon", 5 => "Tue", 6 => "Wed",
        _ => unreachable!(),
    };

    let mut y = 1970_i64;
    let mut remaining = days;
    loop {
        let days_in_year = if is_leap(y) { 366 } else { 365 };
        if remaining < days_in_year { break; }
        remaining -= days_in_year;
        y += 1;
    }

    let months_days = if is_leap(y) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };

    let mut m = 0;
    for &md in months_days.iter() {
        if remaining < md { break; }
        remaining -= md;
        m += 1;
    }
    let day = (remaining + 1) as u8;

    let hour = (local % 86400) / 3600;
    let minute = (local % 3600) / 60;
    let second = local % 60;

    let month_names = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
    let month_str = month_names[m as usize];
    let time_str = format!("{:02}:{:02}:{:02}", hour, minute, second);
    let date_str = format!("{} {} {:02}, {}", weekday, month_str, day, y);
    let uptime = format_uptime();
    let batteries = read_batteries();

    // Compact: time, date, uptime, battery gauge
    let rows_count = 7; // 5 for big time + 1 date + 1 battery
    let chunks = Layout::vertical(
        (0..rows_count).map(|_| Constraint::Length(1)).collect::<Vec<_>>(),
    )
    .split(area);

    // Big Time
    let mut time_lines = vec![String::new(), String::new(), String::new(), String::new(), String::new()];
    for (i, c) in time_str.chars().enumerate() {
        let ch = big_char(c);
        // Add spacing between characters
        let space = if i > 0 { " " } else { "" };
        time_lines[0].push_str(&format!("{}{}", space, ch[0]));
        time_lines[1].push_str(&format!("{}{}", space, ch[1]));
        time_lines[2].push_str(&format!("{}{}", space, ch[2]));
        time_lines[3].push_str(&format!("{}{}", space, ch[3]));
        time_lines[4].push_str(&format!("{}{}", space, ch[4]));
    }
    for i in 0..5 {
        if i < chunks.len() {
            f.render_widget(
                Paragraph::new(Line::from(Span::styled(&time_lines[i], Style::default().fg(theme.accent))))
                    .style(Style::default().bg(theme.bg))
                    .alignment(ratatui::layout::Alignment::Center),
                chunks[i],
            );
        }
    }

    // Date + uptime combined on one line
    if 5 < chunks.len() {
        let dt = format!("{}  ·  {}", date_str, uptime);
        f.render_widget(
            Paragraph::new(Line::from(Span::styled(&dt, Style::default().fg(theme.dim))))
                .style(Style::default().bg(theme.bg))
                .alignment(ratatui::layout::Alignment::Center),
            chunks[5],
        );
    }

    // Battery
    if let Some(bat) = batteries.first() {
        let gauge_color = if bat.capacity < 20 {
            theme.red
        } else if bat.capacity < 40 {
            theme.yellow
        } else {
            theme.green
        };
        let icon = match bat.status.as_str() {
            "Charging" => "⚡",
            "Full" | "Not charging" => "⚡",
            _ => "🔋",
        };
        let label = format!("{} {}%", icon, bat.capacity);
        
        // Draw a tiny text bar: [██████    ]
        let filled = (bat.capacity as usize) / 10;
        let empty = 10 - filled;
        let bar = format!(" {}{}", "█".repeat(filled), "░".repeat(empty));
        
        f.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(label, Style::default().fg(gauge_color)),
                Span::styled(bar, Style::default().fg(gauge_color).bg(theme.surface)),
            ]))
            .alignment(ratatui::layout::Alignment::Center),
            chunks[4],
        );
    } else {
        f.render_widget(
            Paragraph::new(Line::from(Span::styled(" —", Style::default().fg(theme.dim))))
                .style(Style::default().bg(theme.bg)),
            chunks[4],
        );
    }
}
