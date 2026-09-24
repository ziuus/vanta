//! "Up next": the day at a glance. Upcoming agenda events with relative
//! times, followed by open tasks. Read-only; editing lives on the Focus page.

use chrono::{DateTime, Datelike, Local};
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::monitors::{agenda, tasks};
use crate::theme::Theme;
use crate::widgets::meter;

/// Relative label for an event start: "now", "in 12m", "in 3h", "tmrw 09:00",
/// "Fri 09:00", "3 Oct".
fn when(start: DateTime<Local>, end: Option<DateTime<Local>>, now: DateTime<Local>) -> String {
    if start <= now && end.is_some_and(|e| e > now) {
        return "now".into();
    }
    let mins = (start - now).num_minutes();
    let days = (start.date_naive() - now.date_naive()).num_days();
    match (days, mins) {
        (0, m) if m < 60 => format!("in {}m", m.max(0)),
        (0, m) if m < 6 * 60 => format!("in {}h", m / 60),
        (0, _) => start.format("%H:%M").to_string(),
        (1, _) => start.format("tmrw %H:%M").to_string(),
        (2..=6, _) => start.format("%a %H:%M").to_string(),
        _ if start.year() == now.year() => start.format("%-d %b").to_string(),
        _ => start.format("%-d %b %Y").to_string(),
    }
}

fn upcoming(now: DateTime<Local>) -> Vec<agenda::Event> {
    agenda::snapshot()
        .events
        .into_iter()
        .filter(|e| e.end_time.unwrap_or(e.start_time) >= now)
        .collect()
}

/// One-line teaser for the clock: the ongoing or next event today/tomorrow.
pub fn next_note() -> Option<String> {
    let now = Local::now();
    let e = upcoming(now).into_iter().next()?;
    if (e.start_time.date_naive() - now.date_naive()).num_days() > 1 {
        return None;
    }
    let w = when(e.start_time, e.end_time, now);
    Some(if w == "now" {
        format!("now · {}", e.summary)
    } else {
        format!("next · {} {}", e.summary, w)
    })
}

pub fn render(f: &mut Frame, area: Rect, theme: &Theme) {
    if area.height == 0 || area.width < 12 {
        return;
    }
    let now = Local::now();
    let w = area.width as usize;
    let h = area.height as usize;
    let events = upcoming(now);
    let open: Vec<tasks::Task> = tasks::snapshot()
        .tasks
        .into_iter()
        .filter(|t| !t.completed)
        .collect();

    let dim = Style::default().fg(theme.dim);
    let mut lines: Vec<Line> = Vec::new();

    // Give events up to ~60% of rows when tasks also need space.
    let ev_budget = if open.is_empty() {
        h
    } else {
        (h * 3 / 5).max(1)
    };
    for e in events.iter().take(ev_budget) {
        let label = when(e.start_time, e.end_time, now);
        let live = label == "now";
        let soon = label.starts_with("in ") && label.ends_with('m');
        let tag_style = if live {
            Style::default()
                .fg(theme.bg)
                .bg(theme.accent)
                .add_modifier(Modifier::BOLD)
        } else if soon {
            Style::default().fg(theme.yellow)
        } else {
            Style::default().fg(theme.secondary)
        };
        let tag = format!(" {:<10}", label);
        let name_w = w.saturating_sub(tag.chars().count() + 1);
        lines.push(Line::from(vec![
            Span::styled(tag, tag_style),
            Span::raw(" "),
            Span::styled(
                meter::ellipsize(&e.summary, name_w),
                Style::default().fg(theme.text),
            ),
        ]));
    }
    if events.is_empty() && h >= 2 {
        lines.push(Line::from(Span::styled(" nothing scheduled", dim)));
    }

    if !open.is_empty() && lines.len() + 2 <= h {
        lines.push(Line::from(Span::styled(
            "─".repeat(w),
            Style::default().fg(theme.surface),
        )));
        let room = h - lines.len();
        let shown = if open.len() > room {
            room.saturating_sub(1)
        } else {
            open.len()
        };
        for t in open.iter().take(shown) {
            let (mark, col) = if t.urgent {
                ("! ", theme.yellow)
            } else {
                ("○ ", theme.dim)
            };
            lines.push(Line::from(vec![
                Span::styled(format!(" {}", mark), Style::default().fg(col)),
                Span::styled(
                    meter::ellipsize(t.text.trim_start_matches('!').trim(), w.saturating_sub(4)),
                    Style::default().fg(theme.text),
                ),
            ]));
        }
        if shown < open.len() {
            lines.push(Line::from(Span::styled(
                format!("   +{} more tasks", open.len() - shown),
                dim,
            )));
        }
    }

    // Spare rows at the bottom: how far through the day/week/month/year we are.
    let spare = h.saturating_sub(lines.len());
    if spare >= 3 {
        let bars = progress_lines(now, theme, w);
        let n = bars.len().min(spare - 1);
        let pad = h - lines.len() - n;
        lines.extend(std::iter::repeat_n(Line::default(), pad));
        lines.extend(bars.into_iter().take(n));
    }

    f.render_widget(Paragraph::new(lines), area);
}

/// Fraction of the current day, week (Mon-start), month and year elapsed.
fn progress(now: DateTime<Local>) -> [(&'static str, f64); 4] {
    use chrono::Timelike;
    let secs = now.num_seconds_from_midnight() as f64;
    let day = secs / 86_400.0;
    let week = (now.weekday().num_days_from_monday() as f64 + day) / 7.0;
    let days_in_month = {
        let (y, m) = (now.year(), now.month());
        let next = if m == 12 {
            chrono::NaiveDate::from_ymd_opt(y + 1, 1, 1)
        } else {
            chrono::NaiveDate::from_ymd_opt(y, m + 1, 1)
        };
        next.and_then(|n| n.pred_opt()).map_or(30, |d| d.day()) as f64
    };
    let month = (now.day0() as f64 + day) / days_in_month;
    let days_in_year = if chrono::NaiveDate::from_ymd_opt(now.year(), 2, 29).is_some() {
        366.0
    } else {
        365.0
    };
    let year = (now.ordinal0() as f64 + day) / days_in_year;
    [
        ("day", day),
        ("week", week),
        ("month", month),
        ("year", year),
    ]
}

fn progress_lines(now: DateTime<Local>, theme: &Theme, w: usize) -> Vec<Line<'static>> {
    let bar_w = w.saturating_sub(13);
    if bar_w < 6 {
        return Vec::new();
    }
    progress(now)
        .into_iter()
        .map(|(label, frac)| {
            let (on, off) = meter::track(frac, bar_w);
            Line::from(vec![
                Span::styled(format!(" {:<6}", label), Style::default().fg(theme.dim)),
                Span::styled(on, Style::default().fg(theme.accent)),
                Span::styled(off, Style::default().fg(theme.surface)),
                Span::styled(
                    format!(" {:>3.0}%", frac * 100.0),
                    Style::default().fg(theme.text),
                ),
            ])
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn relative_labels() {
        let now = Local.with_ymd_and_hms(2026, 9, 24, 10, 0, 0).unwrap();
        let at = |d, h, m| Local.with_ymd_and_hms(2026, 9, d, h, m, 0).unwrap();
        assert_eq!(when(at(24, 10, 12), None, now), "in 12m");
        assert_eq!(when(at(24, 13, 0), None, now), "in 3h");
        assert_eq!(when(at(24, 18, 30), None, now), "18:30");
        assert_eq!(when(at(25, 9, 0), None, now), "tmrw 09:00");
        assert_eq!(when(at(24, 9, 0), Some(at(24, 11, 0)), now), "now");
        assert_eq!(when(at(27, 9, 0), None, now), "Sun 09:00");
    }

    #[test]
    fn progress_fractions() {
        // Thursday 24 Sep 2026, 12:00.
        let now = Local.with_ymd_and_hms(2026, 9, 24, 12, 0, 0).unwrap();
        let p = progress(now);
        assert!((p[0].1 - 0.5).abs() < 1e-9);
        assert!((p[1].1 - 3.5 / 7.0).abs() < 1e-9);
        assert!((p[2].1 - 23.5 / 30.0).abs() < 1e-9);
        assert!(p[3].1 > 0.72 && p[3].1 < 0.74);
    }
}
