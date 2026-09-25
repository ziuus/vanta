use chrono::Utc;
use chrono_tz::Tz;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph};
use ratatui::Frame;

use crate::theme::Theme;

pub fn render(f: &mut Frame, area: Rect, theme: &Theme, timezones: &[String], h24: bool) {
    if area.height < 3 || timezones.is_empty() {
        return;
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.surface));

    let inner = block.inner(area);
    f.render_widget(block, area);

    if inner.width < 10 {
        return;
    }

    // Divide the inner area into equal columns for each timezone
    let mut constraints = Vec::new();
    for _ in 0..timezones.len() {
        constraints.push(Constraint::Ratio(1, timezones.len() as u32));
    }

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(constraints)
        .split(inner);

    let now = Utc::now();

    for (i, tz_str) in timezones.iter().enumerate() {
        if i >= chunks.len() {
            break;
        }

        let mut label = tz_str.as_str();

        let time_str = if let Ok(tz) = tz_str.parse::<Tz>() {
            let local_time = now.with_timezone(&tz);
            if let Some(city) = tz_str.split('/').next_back() {
                label = city;
            }
            if h24 {
                local_time.format("%H:%M").to_string()
            } else {
                local_time.format("%I:%M %p").to_string()
            }
        } else {
            "Invalid TZ".into()
        };

        // Replace underscores with spaces in label
        let display_label = label.replace('_', " ");

        let p = Paragraph::new(vec![
            Line::from(vec![Span::styled(
                time_str,
                Style::default().fg(theme.text).add_modifier(Modifier::BOLD),
            )]),
            Line::from(vec![Span::styled(
                display_label.to_uppercase(),
                Style::default().fg(theme.dim),
            )]),
        ])
        .alignment(Alignment::Center);

        f.render_widget(p, chunks[i]);
    }
}
