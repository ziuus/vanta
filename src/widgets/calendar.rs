use chrono::{Datelike, Local, NaiveDate};

use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::app;

/// Renders a month calendar with today highlighted.
/// `month_offset` allows Left/Right navigation between months.
pub fn render(f: &mut Frame, area: Rect, theme: &app::Theme, month_offset: i32) {
    let now = Local::now();

    // Compute the target month/year based on offset
    let total_months = now.year() * 12 + now.month() as i32 - 1 + month_offset;
    let year = total_months.div_euclid(12);
    let month = (total_months.rem_euclid(12) + 1) as u32;

    let today = if month_offset == 0 {
        now.day()
    } else {
        0 // not in current month, no "today"
    };

    let first = match NaiveDate::from_ymd_opt(year, month, 1) {
        Some(d) => d,
        None => return, // out-of-range date (extreme month_offset) — render nothing
    };
    let first_weekday = first.weekday().num_days_from_monday(); // 0=Mon
    let days_in_month = days_in_month(year, month);

    // Build weeks as vec of (text, is_today)
    let mut weeks: Vec<Vec<(String, bool)>> = Vec::new();
    let mut week: Vec<(String, bool)> = Vec::new();
    for _ in 0..first_weekday {
        week.push((String::new(), false));
    }
    for day in 1..=days_in_month {
        let is_today = day == today;
        week.push((day.to_string(), is_today));
        if week.len() == 7 && day < days_in_month {
            weeks.push(std::mem::take(&mut week));
        }
    }
    if !week.is_empty() {
        while week.len() < 7 {
            week.push((String::new(), false));
        }
        weeks.push(week);
    }

    // Navigation hint
    let nav = if month_offset == 0 {
        " Today".to_string()
    } else {
        format!(
            " {} {:02}",
            if month_offset < 0 { '◀' } else { '▶' },
            month_offset.abs()
        )
    };

    // Build styled lines
    let mut lines: Vec<Line> = Vec::new();

    // Month/year header with offset indicator
    let header = format!("{} {}", month_name(month), year);
    let nav_display = format!(" {}", nav);

    // We want the header to be centered within the 20-char calendar width
    let total_header_len = header.len() + nav_display.len();
    let header_pad = if total_header_len < 20 {
        " ".repeat((20 - total_header_len) / 2)
    } else {
        String::new()
    };

    lines.push(Line::from(vec![
        Span::styled(header_pad, Style::default().bg(theme.surface)),
        Span::styled(
            header,
            Style::default().fg(if month_offset == 0 {
                theme.accent
            } else {
                theme.secondary
            }),
        ),
        Span::styled(nav_display, Style::default().fg(theme.dim)),
    ]));

    lines.push(Line::from(Span::styled(
        "Mo Tu We Th Fr Sa Su",
        Style::default().fg(theme.dim),
    )));

    for w in &weeks {
        let mut spans = Vec::new();
        for (i, (text, is_today)) in w.iter().enumerate() {
            if i > 0 {
                spans.push(Span::styled(" ", Style::default().fg(theme.text)));
            }
            if *is_today && month_offset == 0 {
                spans.push(Span::styled(
                    format!("{:>2}", text),
                    Style::default()
                        .fg(theme.bg)
                        .bg(theme.accent)
                        .add_modifier(ratatui::style::Modifier::BOLD),
                ));
            } else if text.is_empty() {
                spans.push(Span::styled("  ", Style::default().fg(theme.dim)));
            } else {
                spans.push(Span::styled(
                    format!("{:>2}", text),
                    Style::default().fg(theme.dim),
                ));
            }
        }
        lines.push(Line::from(spans));
    }

    // Center the calendar
    let cal_width = 20; // "Mo Tu We Th Fr Sa Su" is 20 chars
    let padding = if area.width > cal_width {
        ((area.width - cal_width) / 2) as usize
    } else {
        0
    };
    let pad_str = " ".repeat(padding);

    // Apply padding to all lines
    for line in &mut lines {
        if !pad_str.is_empty() {
            line.spans.insert(0, Span::raw(pad_str.clone()));
        }
    }

    f.render_widget(
        Paragraph::new(lines).style(Style::default().bg(theme.surface)),
        area,
    );
}

fn month_name(m: u32) -> &'static str {
    match m {
        1 => "January",
        2 => "February",
        3 => "March",
        4 => "April",
        5 => "May",
        6 => "June",
        7 => "July",
        8 => "August",
        9 => "September",
        10 => "October",
        11 => "November",
        12 => "December",
        _ => "?",
    }
}

fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0) {
                29
            } else {
                28
            }
        }
        _ => 30,
    }
}
