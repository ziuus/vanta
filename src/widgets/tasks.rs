use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Block, Borders};
use ratatui::Frame;

use crate::theme::Theme;
use crate::monitors::tasks;

pub fn render(f: &mut Frame, area: Rect, theme: &Theme) {
    if area.height < 3 || area.width < 15 {
        return;
    }

    let snap = tasks::snapshot();
    let mut lines = Vec::new();

    if snap.tasks.is_empty() {
        lines.push(Line::from(vec![
            Span::styled(" No tasks found.", Style::default().fg(theme.dim)),
        ]));
        lines.push(Line::from(vec![
            Span::styled(" Edit ~/.config/vanta/todo.md", Style::default().fg(theme.dim)),
        ]));
    } else {
        // Table header
        lines.push(Line::from(vec![
            Span::styled("   STATUS  TASK", Style::default().fg(theme.dim)),
        ]));
        lines.push(Line::from(vec![])); // empty line for spacing

        let display_count = (area.height.saturating_sub(2)) as usize;
        
        for task in snap.tasks.iter().take(display_count) {
            let (icon, color, text_style) = if task.completed {
                (" ✓ ", theme.dim, Style::default().fg(theme.dim).add_modifier(Modifier::CROSSED_OUT))
            } else if task.urgent {
                (" ! ", theme.red, Style::default().fg(theme.text).add_modifier(Modifier::BOLD))
            } else {
                (" ◯ ", theme.accent, Style::default().fg(theme.text))
            };

            lines.push(Line::from(vec![
                Span::styled(icon, Style::default().fg(color)),
                Span::styled(format!(" {}", task.text), text_style),
            ]));
        }
    }

    let paragraph = Paragraph::new(lines).block(Block::default().borders(Borders::NONE));
    f.render_widget(paragraph, area);
}
