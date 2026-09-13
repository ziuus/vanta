use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use chrono::Local;

use crate::monitors::agenda;
use crate::theme::Theme;

pub fn render(f: &mut Frame, area: Rect, theme: &Theme) {
    if area.height < 3 || area.width < 20 {
        return;
    }

    let snap = agenda::snapshot();
    let mut lines = Vec::new();

    if snap.events.is_empty() {
        lines.push(Line::from(vec![Span::styled(
            " No upcoming events.",
            Style::default().fg(theme.dim),
        )]));
        lines.push(Line::from(vec![Span::styled(
            " Sync ~/.config/vanta/agenda.ics",
            Style::default().fg(theme.dim),
        )]));
    } else {
        // Table header
        lines.push(Line::from(vec![
            Span::styled("   TIME", Style::default().fg(theme.dim)),
            Span::raw(" ".repeat(area.width.saturating_sub(13) as usize)),
            Span::styled("EVENT", Style::default().fg(theme.dim)),
        ]));
        lines.push(Line::from(vec![])); // empty line for spaciousness

        let display_count = (area.height.saturating_sub(2) / 2) as usize; // double spacing

        let now = Local::now();

        for event in snap.events.iter().take(display_count) {
            let is_active =
                event.start_time <= now && event.end_time.unwrap_or(event.start_time) >= now;
            let time_str = if is_active {
                "NOW".to_string()
            } else {
                let diff = event.start_time.signed_duration_since(now);
                if diff.num_days() == 0 {
                    if diff.num_hours() == 0 {
                        format!("in {}m", diff.num_minutes())
                    } else {
                        format!("in {}h", diff.num_hours())
                    }
                } else if diff.num_days() == 1 {
                    "tomorrow".to_string()
                } else {
                    format!("in {}d", diff.num_days())
                }
            };

            let title_len = event.summary.len();
            let time_len = time_str.len();
            let pad = area.width.saturating_sub((title_len + time_len + 4) as u16) as usize;

            let (time_color, title_color) = if is_active {
                (theme.accent, theme.text)
            } else {
                (theme.dim, theme.text)
            };

            lines.push(Line::from(vec![
                Span::styled(format!("  {}", time_str), Style::default().fg(time_color)),
                Span::raw(" ".repeat(pad)),
                Span::styled(
                    format!("{} ", event.summary),
                    Style::default().fg(title_color),
                ),
            ]));

            lines.push(Line::from(vec![])); // double spaced
        }
    }

    let paragraph = Paragraph::new(lines).block(Block::default().borders(Borders::NONE));
    f.render_widget(paragraph, area);
}
