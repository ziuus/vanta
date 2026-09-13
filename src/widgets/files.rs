use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;

use crate::monitors::files;

pub fn render(f: &mut Frame, area: Rect, theme: &crate::theme::Theme, is_focused: bool, selected_idx: &mut usize) {
    let snap = files::snapshot();
    

    // Split 40% list, 60% preview
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .spacing(1)
        .split(area);

    let list_area = chunks[0];
    let preview_area = chunks[1];

    let mut list_lines = Vec::new();
    let num_items = snap.items.len();

    // Adjust selected index
    let max_idx = num_items.saturating_sub(1);
    *selected_idx = (*selected_idx).min(max_idx);
    let selected = *selected_idx;

    // Header: Current directory path
    let cur_dir = snap.current_dir.to_string_lossy();
    list_lines.push(Line::from(vec![
        Span::styled(format!(" 📁 {}", cur_dir), Style::default().fg(theme.accent)),
    ]));
    list_lines.push(Line::from(""));

    // Items
    for (i, item) in snap.items.iter().enumerate() {
        let prefix = if i == selected { " > " } else { "   " };
        let icon = if item.is_dir { "📁" } else { "📄" };
        let style = if i == selected {
            Style::default().fg(theme.accent)
        } else if item.is_dir {
            Style::default().fg(theme.text)
        } else {
            Style::default().fg(theme.dim)
        };
        list_lines.push(Line::from(vec![
            Span::styled(prefix, Style::default().fg(theme.accent)),
            Span::styled(format!("{} ", icon), style),
            Span::styled(&item.name, style),
        ]));
    }

    let border_color = if is_focused { theme.accent } else { theme.surface };
    f.render_widget(
        Paragraph::new(list_lines)
            .block(Block::default().borders(Borders::RIGHT).border_style(Style::default().fg(border_color))),
        list_area,
    );

    // Preview
    let mut preview_lines = Vec::new();
    if num_items > 0 {
        if let Some(preview) = &snap.preview {
            for line in preview.lines() {
                preview_lines.push(Line::from(Span::styled(line, Style::default().fg(theme.text))));
            }
        }
    }

    f.render_widget(
        Paragraph::new(preview_lines).wrap(Wrap { trim: false }),
        preview_area,
    );
}
