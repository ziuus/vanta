use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Gauge, Paragraph};
use ratatui::Frame;

use crate::app;

#[derive(Debug)]
struct Battery {
    capacity: u8,
    status: String,
    power_w: f64,
    energy_now_wh: f64,
    energy_full_wh: f64,
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

        let power_now = fs::read_to_string(base.join("power_now"))
            .ok()
            .and_then(|s| s.trim().parse::<f64>().ok())
            .unwrap_or(0.0);

        let energy_now = fs::read_to_string(base.join("energy_now"))
            .ok()
            .and_then(|s| s.trim().parse::<f64>().ok())
            .unwrap_or(0.0);

        let energy_full = fs::read_to_string(base.join("energy_full"))
            .ok()
            .and_then(|s| s.trim().parse::<f64>().ok())
            .unwrap_or(0.0);

        bats.push(Battery {
            capacity,
            status,
            power_w: power_now / 1_000_000.0,
            energy_now_wh: energy_now / 1_000_000.0,
            energy_full_wh: energy_full / 1_000_000.0,
        });
    }

    bats
}

fn format_time_remaining(hours: f64) -> String {
    if hours <= 0.0 {
        return "—".to_string();
    }
    let total_mins = (hours * 60.0) as u64;
    let h = total_mins / 60;
    let m = total_mins % 60;
    if h > 0 {
        format!("{}h {:02}m", h, m)
    } else {
        format!("{}m", m)
    }
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
        format!("up {}d {}h {}m", days, hours, mins)
    } else if hours > 0 {
        format!("up {}h {}m", hours, mins)
    } else {
        format!("up {}m", mins)
    }
}

fn is_leap(year: i64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

pub fn render(f: &mut Frame, area: Rect, theme: &app::Theme, demo: bool) {
    let now = if demo {
        // Freeze time at 2026-07-08 14:30:00 UTC+5:30
        std::time::Duration::from_secs(1783501200)
    } else {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
    };

    let secs = now.as_secs();
    let ist_offset = 5 * 3600 + 30 * 60;
    let local = (secs + ist_offset) as i64;
    let days = local / 86400;

    let weekday = match days.rem_euclid(7) {
        0 => "Thu",
        1 => "Fri",
        2 => "Sat",
        3 => "Sun",
        4 => "Mon",
        5 => "Tue",
        6 => "Wed",
        _ => unreachable!(),
    };

    let mut y = 1970_i64;
    let mut remaining = days;
    loop {
        let days_in_year = if is_leap(y) { 366 } else { 365 };
        if remaining < days_in_year {
            break;
        }
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
        if remaining < md {
            break;
        }
        remaining -= md;
        m += 1;
    }
    let month = m + 1;
    let day = (remaining + 1) as u8;

    let hour = (local % 86400) / 3600;
    let minute = (local % 3600) / 60;
    let second = local % 60;

    let time_str = format!("{:02}:{:02}:{:02}", hour, minute, second);
    let date_str = format!("{}, {} {:02}, {}", weekday, month, day, y);
    let uptime = format_uptime();
    let batteries = read_batteries();

    // Layout: time/date rows + battery section
    let mut rows = vec![Constraint::Length(1); 3]; // time, date, uptime

    // Battery section: each battery gets a gauge row + label, then a summary row
    let bat_count = batteries.len().max(1);
    // For each battery: 1 line label + 1 line gauge = 2 lines per battery
    let bat_lines = bat_count * 2;
    // Add a summary line if multiple batteries
    let bat_total = bat_lines + if bat_count > 1 { 1 } else { 0 };

    for _ in 0..bat_total {
        rows.push(Constraint::Length(1));
    }

    // If no batteries found, add one "no battery" line
    if batteries.is_empty() {
        rows.push(Constraint::Length(1));
    }

    let chunks = Layout::vertical(rows).split(area);

    // Time
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            &time_str,
            Style::default().fg(theme.text),
        )))
        .style(Style::default().bg(theme.bg)),
        chunks[0],
    );

    // Date
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            &date_str,
            Style::default().fg(theme.dim),
        )))
        .style(Style::default().bg(theme.bg)),
        chunks[1],
    );

    // Uptime
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            &uptime,
            Style::default().fg(theme.dim),
        )))
        .style(Style::default().bg(theme.bg)),
        chunks[2],
    );

    // Battery section
    let mut row_idx = 3;

    if batteries.is_empty() {
        f.render_widget(
            Paragraph::new(Line::from(Span::styled(
                " no battery",
                Style::default().fg(theme.dim),
            )))
            .style(Style::default().bg(theme.bg)),
            chunks[row_idx],
        );
        return;
    }

    for bat in &batteries {
        if row_idx >= chunks.len() {
            break;
        }

        let gauge_color = if bat.capacity < 20 {
            theme.red
        } else if bat.capacity < 40 {
            theme.yellow
        } else {
            theme.green
        };

        let icon = match bat.status.as_str() {
            "Charging" => "🔌",
            "Discharging" => "🔋",
            "Full" | "Not charging" => "⚡",
            _ => "🔋",
        };

        // Time remaining estimate
        let time_remaining = if bat.status == "Discharging" && bat.power_w > 0.0 {
            let hours = bat.energy_now_wh / bat.power_w;
            format_time_remaining(hours)
        } else if bat.status == "Charging" && bat.power_w > 0.0 {
            let remaining_wh = bat.energy_full_wh - bat.energy_now_wh;
            if remaining_wh > 0.0 {
                let hours = remaining_wh / bat.power_w;
                format!("{} until full", format_time_remaining(hours))
            } else {
                "—".to_string()
            }
        } else {
            "—".to_string()
        };

        // Label line: icon, name, capacity %, status, time remaining
        f.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(format!(" {}  {}% ", icon, bat.capacity), Style::default().fg(gauge_color)),
                Span::styled(&time_remaining, Style::default().fg(theme.dim)),
            ]))
            .style(Style::default().bg(theme.bg)),
            chunks[row_idx],
        );
        row_idx += 1;

        if row_idx >= chunks.len() {
            break;
        }

        // Gauge row
        f.render_widget(
            Gauge::default()
                .gauge_style(Style::default().fg(gauge_color).bg(theme.surface))
                .percent(bat.capacity.min(100) as u16)
                .label(String::new()),
            chunks[row_idx],
        );
        row_idx += 1;
    }

    // Multi-battery summary
    if batteries.len() > 1 && row_idx < chunks.len() {
        let total_cap: u8 = batteries.iter().map(|b| b.capacity).sum::<u8>() / batteries.len() as u8;
        let total_color = if total_cap < 20 {
            theme.red
        } else if total_cap < 40 {
            theme.yellow
        } else {
            theme.green
        };
        f.render_widget(
            Paragraph::new(Line::from(Span::styled(
                format!(" ⚡ avg {}%", total_cap),
                Style::default().fg(total_color),
            )))
            .style(Style::default().bg(theme.bg)),
            chunks[row_idx],
        );
    }
}
