use chrono::{Datelike, Local, NaiveDate};

use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::app;

/// lvsk-style month calendar: ISO week numbers down the left, weekday header,
/// today marked with a diamond. `month_offset` steps months via Left/Right.
pub fn render(f: &mut Frame, area: Rect, theme: &app::Theme, month_offset: i32) {
    let now = Local::now();

    let total_months = now.year() * 12 + now.month() as i32 - 1 + month_offset;
    let year = total_months.div_euclid(12);
    let month = (total_months.rem_euclid(12) + 1) as u32;

    let today = if month_offset == 0 { now.day() } else { 0 };

    let first = match NaiveDate::from_ymd_opt(year, month, 1) {
        Some(d) => d,
        None => return, // out-of-range date (extreme month_offset)
    };
    let first_weekday = first.weekday().num_days_from_monday(); // 0=Mon
    let days = days_in_month(year, month);

    // Rows of 7 day-cells, padded at both ends; each row carries its ISO week.
    let mut weeks: Vec<(u32, Vec<u32>)> = Vec::new();
    let mut week: Vec<u32> = vec![0; first_weekday as usize];
    for day in 1..=days {
        week.push(day);
        if week.len() == 7 {
            let iso = NaiveDate::from_ymd_opt(year, month, day)
                .map(|d| d.iso_week().week())
                .unwrap_or(0);
            weeks.push((iso, std::mem::take(&mut week)));
        }
    }
    if !week.is_empty() {
        let last = *week.iter().rev().find(|d| **d > 0).unwrap_or(&1);
        let iso = NaiveDate::from_ymd_opt(year, month, last)
            .map(|d| d.iso_week().week())
            .unwrap_or(0);
        while week.len() < 7 {
            week.push(0);
        }
        weeks.push((iso, week));
    }

    // "wk " + 7 cells of "dd " => 3 + 21 = 24 wide.
    const CAL_W: u16 = 24;
    let mut lines: Vec<Line> = Vec::new();

    // Header: month + year, with a nav indicator when browsing other months.
    let title = format!("{} {}", month_name(month), year);
    let nav = if month_offset == 0 {
        String::new()
    } else {
        format!(" {}{}", if month_offset < 0 { '\u{25c0}' } else { '\u{25b6}' }, month_offset.abs())
    };
    let title_col = if month_offset == 0 {
        theme.accent
    } else {
        theme.secondary
    };
    let head_pad = (CAL_W as usize).saturating_sub(title.len() + nav.len()) / 2;
    lines.push(Line::from(vec![
        Span::raw(" ".repeat(head_pad)),
        Span::styled(
            title,
            Style::default()
                .fg(title_col)
                .add_modifier(ratatui::style::Modifier::BOLD),
        ),
        Span::styled(nav, Style::default().fg(theme.dim)),
    ]));
    lines.push(Line::from(""));

    // Weekday header, aligned past the week-number gutter.
    let mut hdr = vec![Span::styled("wk ", Style::default().fg(theme.dim))];
    for d in ["mo", "tu", "we", "th", "fr", "sa", "su"] {
        hdr.push(Span::styled(
            format!("{:>2} ", d),
            Style::default().fg(theme.secondary),
        ));
    }
    lines.push(Line::from(hdr));

    for (iso, w) in &weeks {
        let mut spans = vec![Span::styled(
            format!("{:>2} ", iso),
            Style::default().fg(theme.dim),
        )];
        for day in w {
            if *day == 0 {
                spans.push(Span::raw("   "));
                continue;
            }
            let is_today = *day == today && month_offset == 0;
            if is_today {
                // Diamond marker + inverse fill, so today reads at a glance.
                spans.push(Span::styled(
                    format!("{:>2}", day),
                    Style::default()
                        .fg(theme.bg)
                        .bg(theme.accent)
                        .add_modifier(ratatui::style::Modifier::BOLD),
                ));
                spans.push(Span::styled(
                    "\u{25c6}",
                    Style::default().fg(theme.accent),
                ));
            } else {
                spans.push(Span::styled(
                    format!("{:>2} ", day),
                    Style::default().fg(theme.text),
                ));
            }
        }
        lines.push(Line::from(spans));
    }

    // Centre the whole block horizontally in the panel.
    let pad = (area.width.saturating_sub(CAL_W) / 2) as usize;
    if pad > 0 {
        for line in &mut lines {
            line.spans.insert(0, Span::raw(" ".repeat(pad)));
        }
    }

    let top = (area.height.saturating_sub(lines.len() as u16)) / 2;
    f.render_widget(
        Paragraph::new(lines),
        Rect::new(area.x, area.y + top, area.width, area.height - top),
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
