use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::monitors::tasks;
use crate::theme::Theme;

pub fn render(f: &mut Frame, area: Rect, theme: &Theme, is_focused: bool, selected: usize) {
    if area.height < 3 || area.width < 15 {
        return;
    }

    let snap = tasks::snapshot();
    let mut lines = Vec::new();

    if snap.tasks.is_empty() {
        lines.push(Line::from(vec![Span::styled(
            " No tasks found.",
            Style::default().fg(theme.dim),
        )]));
        lines.push(Line::from(vec![Span::styled(
            " Press 'e' or Enter to edit ~/.config/vanta/todo.md",
            Style::default().fg(theme.dim),
        )]));
    } else {
        let display_count = (area.height.saturating_sub(1)) as usize;
        let start_idx = if selected >= display_count {
            selected.saturating_sub(display_count - 1)
        } else {
            0
        };

        for (i, task) in snap.tasks.iter().enumerate().skip(start_idx).take(display_count) {
            let is_sel = is_focused && i == selected;

            let prefix = if is_sel {
                Span::styled(" > ", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD))
            } else {
                Span::raw("   ")
            };

            let (icon, color, text_style) = if task.completed {
                (
                    "[✓]",
                    theme.dim,
                    Style::default()
                        .fg(theme.dim)
                        .add_modifier(Modifier::CROSSED_OUT),
                )
            } else if task.urgent {
                (
                    "[!]",
                    theme.red,
                    Style::default()
                        .fg(if is_sel { theme.accent } else { theme.text })
                        .add_modifier(Modifier::BOLD),
                )
            } else {
                (
                    "[ ]",
                    if is_sel { theme.accent } else { theme.dim },
                    Style::default().fg(if is_sel { theme.accent } else { theme.text }),
                )
            };

            lines.push(Line::from(vec![
                prefix,
                Span::styled(icon, Style::default().fg(color).add_modifier(Modifier::BOLD)),
                Span::styled(format!(" {}", task.text), text_style),
            ]));
        }
    }

    let paragraph = Paragraph::new(lines).block(Block::default().borders(Borders::NONE));
    f.render_widget(paragraph, area);
}
