use chrono::{Datelike, Local, NaiveDate};

use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::theme::Theme;

#[derive(Clone, Copy)]
enum DayKind {
    Prev(u32),
    Current(u32),
    Next(u32),
}

pub fn render(f: &mut Frame, area: Rect, theme: &Theme, month_offset: i32) {
    if area.height < 5 || area.width < 15 {
        return;
    }

    let now = Local::now();
    let total_months = now.year() * 12 + now.month() as i32 - 1 + month_offset;
    let year = total_months.div_euclid(12);
    let month = (total_months.rem_euclid(12) + 1) as u32;
    let today = if month_offset == 0 { now.day() } else { 0 };

    let first = match NaiveDate::from_ymd_opt(year, month, 1) {
        Some(d) => d,
        None => return,
    };
    let first_weekday = first.weekday().num_days_from_monday(); // 0 = Mon, 6 = Sun
    let days = days_in_month(year, month);

    let prev_month_days = if month == 1 {
        days_in_month(year - 1, 12)
    } else {
        days_in_month(year, month - 1)
    };

    let mut weeks: Vec<Vec<DayKind>> = Vec::new();
    let mut current_week: Vec<DayKind> = Vec::with_capacity(7);

    // Fill previous month days
    if first_weekday > 0 {
        let start_day = prev_month_days - first_weekday + 1;
        for d in start_day..=prev_month_days {
            current_week.push(DayKind::Prev(d));
        }
    }

    // Fill current month days
    for day in 1..=days {
        current_week.push(DayKind::Current(day));
        if current_week.len() == 7 {
            weeks.push(std::mem::take(&mut current_week));
        }
    }

    // Fill next month days
    if !current_week.is_empty() {
        let mut next_d = 1;
        while current_week.len() < 7 {
            current_week.push(DayKind::Next(next_d));
            next_d += 1;
        }
        weeks.push(current_week);
    }

    let is_wide = area.width >= 30;
    let mut lines: Vec<Line> = Vec::new();

    // 1. Month / Year Title
    let title = if month_offset != 0 {
        format!("‹ {} {} ›", month_name(month), year)
    } else {
        format!("{} {}", month_name(month), year)
    };

    lines.push(Line::from(vec![
        Span::styled(
            title,
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        ),
    ]));

    // 2. Optional divider if vertical space allows
    if area.height >= 10 {
        let sep_len = if is_wide { 28 } else { 21 };
        lines.push(Line::from(Span::styled(
            "─".repeat(sep_len),
            Style::default().fg(theme.surface),
        )));
    }

    // 3. Day of week headers
    let mut hdr_spans: Vec<Span> = Vec::with_capacity(7);
    let day_names = ["Mo", "Tu", "We", "Th", "Fr", "Sa", "Su"];
    for (i, d) in day_names.iter().enumerate() {
        let is_weekend = i >= 5;
        let col = if is_weekend {
            theme.secondary
        } else {
            theme.dim
        };
        let txt = if is_wide {
            format!(" {:>2} ", d)
        } else {
            format!("{:>2} ", d)
        };
        hdr_spans.push(Span::styled(
            txt,
            Style::default().fg(col).add_modifier(Modifier::BOLD),
        ));
    }
    lines.push(Line::from(hdr_spans));

    // 4. Weeks
    for week in &weeks {
        let mut week_spans: Vec<Span> = Vec::with_capacity(14);
        for (i, day_kind) in week.iter().enumerate() {
            let is_weekend = i >= 5;
            match *day_kind {
                DayKind::Prev(d) | DayKind::Next(d) => {
                    let txt = if is_wide {
                        format!(" {:>2} ", d)
                    } else {
                        format!("{:>2} ", d)
                    };
                    week_spans.push(Span::styled(txt, Style::default().fg(theme.surface)));
                }
                DayKind::Current(d) => {
                    let is_today = d == today && month_offset == 0;
                    if is_today {
                        if is_wide {
                            week_spans.push(Span::raw(" "));
                            week_spans.push(Span::styled(
                                format!("{:>2}", d),
                                Style::default()
                                    .fg(theme.bg)
                                    .bg(theme.accent)
                                    .add_modifier(Modifier::BOLD),
                            ));
                            week_spans.push(Span::raw(" "));
                        } else {
                            week_spans.push(Span::styled(
                                format!("{:>2}", d),
                                Style::default()
                                    .fg(theme.bg)
                                    .bg(theme.accent)
                                    .add_modifier(Modifier::BOLD),
                            ));
                            week_spans.push(Span::raw(" "));
                        }
                    } else {
                        let col = if is_weekend {
                            theme.secondary
                        } else {
                            theme.text
                        };
                        let txt = if is_wide {
                            format!(" {:>2} ", d)
                        } else {
                            format!("{:>2} ", d)
                        };
                        week_spans.push(Span::styled(txt, Style::default().fg(col)));
                    }
                }
            }
        }
        lines.push(Line::from(week_spans));
    }

    let top = (area.height.saturating_sub(lines.len() as u16)) / 2;
    let render_area = Rect::new(
        area.x,
        area.y + top,
        area.width,
        area.height.saturating_sub(top),
    );

    f.render_widget(
        Paragraph::new(lines).alignment(Alignment::Center),
        render_area,
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
