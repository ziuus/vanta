use ratatui::layout::{Rect, Alignment};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Block, Borders};
use ratatui::Frame;

use chrono::Utc;

use crate::theme::Theme;
use crate::monitors::news;

pub fn render(f: &mut Frame, area: Rect, theme: &Theme) {
    if area.height < 3 || area.width < 20 {
        return;
    }

    let snap = news::snapshot();
    let mut lines = Vec::new();

    if !snap.ready {
        let top = area.height.saturating_sub(1) / 2;
        for _ in 0..top {
            lines.push(Line::from(""));
        }
        lines.push(Line::from(vec![
            Span::styled(" Fetching news...", Style::default().fg(theme.dim)),
        ]));
        let paragraph = Paragraph::new(lines)
            .block(Block::default().borders(Borders::NONE))
            .alignment(Alignment::Center);
        f.render_widget(paragraph, area);
        return;
    }

    if snap.items.is_empty() {
        lines.push(Line::from(vec![
            Span::styled(" No news available.", Style::default().fg(theme.dim)),
        ]));
    } else {
        lines.push(Line::from(vec![])); // empty top line for spaciousness

        let display_count = (area.height.saturating_sub(1) / 2).max(1).min(3) as usize; 
        
        let now = Utc::now();

        for item in snap.items.iter().take(display_count) {
            let time_str = if let Some(pub_date) = item.published_at {
                let diff = now.signed_duration_since(pub_date);
                if diff.num_hours() == 0 {
                    format!("{}m ago", diff.num_minutes().max(1))
                } else if diff.num_days() == 0 {
                    format!("{}h ago", diff.num_hours())
                } else {
                    format!("{}d ago", diff.num_days())
                }
            } else {
                "".to_string()
            };

            // Calculate max width for the title
            let marker = " • ";
            let time_len = if time_str.is_empty() { 0 } else { time_str.len() + 3 }; // " (2h ago)"
            let max_title_width = (area.width as usize).saturating_sub(marker.len() + time_len + 1);
            
            let mut title = item.title.clone();
            if title.len() > max_title_width {
                title.truncate(max_title_width.saturating_sub(3));
                title.push_str("...");
            }

            let mut spans = vec![
                Span::styled(marker, Style::default().fg(theme.accent)),
                Span::styled(title, Style::default().fg(theme.text).add_modifier(Modifier::BOLD)),
            ];

            if !time_str.is_empty() {
                spans.push(Span::styled(format!(" ({})", time_str), Style::default().fg(theme.dim)));
            }

            lines.push(Line::from(spans));
            lines.push(Line::from(vec![])); // double spaced
        }
    }

    let paragraph = Paragraph::new(lines).block(Block::default().borders(Borders::NONE));
    f.render_widget(paragraph, area);
}
