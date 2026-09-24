use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use chrono::Local;

use crate::monitors::agenda;
use crate::theme::Theme;

pub fn render(
    f: &mut Frame,
    area: Rect,
    theme: &Theme,
    is_focused: bool,
    selected: usize,
    input_active: bool,
    input_text: &str,
) {
    if area.height < 3 || area.width < 20 {
        return;
    }

    let snap = agenda::snapshot();
    let mut lines = Vec::new();

    let display_reserve = if input_active { 3 } else { 2 };
    let display_count = ((area.height.saturating_sub(display_reserve)) / 2).max(1) as usize;

    if snap.events.is_empty() {
        if !input_active {
            lines.push(Line::from(vec![Span::styled(
                " Nothing scheduled.",
                Style::default().fg(theme.dim),
            )]));
            lines.push(Line::from(vec![Span::styled(
                " a add · e edit agenda.ics",
                Style::default().fg(theme.dim),
            )]));
        }
    } else {
        // Table header
        lines.push(Line::from(vec![
            Span::styled("   TIME", Style::default().fg(theme.dim)),
            Span::raw(" ".repeat((area.width as usize).saturating_sub(13))),
            Span::styled("EVENT", Style::default().fg(theme.dim)),
        ]));
        lines.push(Line::from(vec![])); // empty line for spaciousness

        let start_idx = if selected >= display_count && display_count > 0 {
            selected.saturating_sub(display_count - 1)
        } else {
            0
        };

        let now = Local::now();

        for (i, event) in snap
            .events
            .iter()
            .enumerate()
            .skip(start_idx)
            .take(display_count)
        {
            let is_sel = is_focused && i == selected && !input_active;
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

            let prefix = if is_sel {
                Span::styled(
                    " > ",
                    Style::default()
                        .fg(theme.accent)
                        .add_modifier(Modifier::BOLD),
                )
            } else {
                Span::raw("   ")
            };

            let (time_color, title_color) = if is_active {
                (theme.accent, if is_sel { theme.accent } else { theme.text })
            } else {
                (
                    if is_sel { theme.accent } else { theme.dim },
                    if is_sel { theme.accent } else { theme.text },
                )
            };

            let title_len = event.summary.chars().count();
            let time_len = time_str.chars().count();
            let pad = (area.width as usize).saturating_sub(3 + time_len + title_len + 2);

            lines.push(Line::from(vec![
                prefix,
                Span::styled(time_str, Style::default().fg(time_color)),
                Span::raw(" ".repeat(pad)),
                Span::styled(
                    format!("{} ", event.summary),
                    Style::default().fg(title_color).add_modifier(if is_sel {
                        Modifier::BOLD
                    } else {
                        Modifier::empty()
                    }),
                ),
            ]));

            lines.push(Line::from(vec![])); // double spaced
        }
    }

    if input_active {
        lines.push(Line::from(vec![
            Span::styled(
                " + ",
                Style::default()
                    .fg(theme.green)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("{}▏", input_text),
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            ),
        ]));
    }

    let paragraph = Paragraph::new(lines).block(Block::default().borders(Borders::NONE));
    f.render_widget(paragraph, area);
}
