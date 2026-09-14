use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::monitors::tasks;
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
    if area.height < 3 || area.width < 15 {
        return;
    }

    let snap = tasks::snapshot();
    let mut lines = Vec::new();

    let display_reserve = if input_active { 2 } else { 1 };
    let display_count = (area.height.saturating_sub(display_reserve)) as usize;

    if snap.tasks.is_empty() && !input_active {
        lines.push(Line::from(vec![Span::styled(
            " No tasks found.",
            Style::default().fg(theme.dim),
        )]));
        lines.push(Line::from(vec![Span::styled(
            " Press 'a' to add a task, or 'e' to edit todo.md",
            Style::default().fg(theme.dim),
        )]));
    } else {
        let start_idx = if selected >= display_count && display_count > 0 {
            selected.saturating_sub(display_count - 1)
        } else {
            0
        };

        for (i, task) in snap
            .tasks
            .iter()
            .enumerate()
            .skip(start_idx)
            .take(display_count)
        {
            let is_sel = is_focused && i == selected && !input_active;

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
                Span::styled(
                    icon,
                    Style::default().fg(color).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!(
                        " {}",
                        crate::widgets::meter::ellipsize(
                            &task.text,
                            (area.width as usize).saturating_sub(8)
                        )
                    ),
                    text_style,
                ),
            ]));
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
