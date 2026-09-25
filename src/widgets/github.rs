use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph};
use ratatui::Frame;

use crate::monitors::github;
use crate::theme::Theme;

pub fn render(f: &mut Frame, area: Rect, theme: &Theme) {
    if area.height < 5 || area.width < 10 {
        return;
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.surface));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let snap = github::snapshot();

    // We want to draw a 7-row tall grid, but the terminal only has `inner.height`
    // Wait, GitHub graph is 7 rows (Sunday to Saturday).
    // If we use braille characters, we can fit 4 rows of pixels in 1 character height.
    // 7 rows can be drawn in 2 character lines!
    // But braille is a bit complex for a simple component.
    // What if we just use block characters, e.g. ▄, ▀, █ for two rows per line?
    // 7 rows / 2 = 4 terminal lines!

    // Actually, simple colored blocks (one cell = one day) if we have 7 lines.
    // But if we don't have 7 lines, we can just draw the last N days in a single line, or a mini sparkline!
    // The prompt says "A small bento-box widget that fetches and displays your GitHub "green squares" graph for the current month".

    // Let's just draw the last 4-5 weeks, column by column.
    // 5 weeks * 7 days = 35 days.

    let mut lines = Vec::new();

    // Title / stats
    lines.push(Line::from(vec![
        Span::styled(
            " GitHub",
            Style::default().fg(theme.text).add_modifier(Modifier::BOLD),
        ),
        Span::raw(format!("  {} streak", snap.streak)),
    ]));

    // Grid
    let cols = (inner.width as usize).min(snap.contributions.len() / 7);
    if cols > 0 && inner.height >= 9 {
        // We have space for a full 7-row grid
        let start_week = (snap.contributions.len() / 7).saturating_sub(cols);

        for day_of_week in 0..7 {
            let mut line = String::new();
            for w in 0..cols {
                let idx = (start_week + w) * 7 + day_of_week;
                if idx < snap.contributions.len() {
                    let count = snap.contributions[idx];
                    let c = if count > 10 {
                        '█'
                    } else if count > 5 {
                        '▓'
                    } else if count > 0 {
                        '▒'
                    } else {
                        '·'
                    };
                    line.push(c);
                    line.push(' ');
                }
            }
            lines.push(Line::from(Span::styled(
                line,
                Style::default().fg(theme.green),
            )));
        }
    } else {
        // Not enough height, draw a sparkline or horizontal list
        let mut line = String::new();
        let days = inner.width.min(snap.contributions.len() as u16) as usize;
        let start = snap.contributions.len().saturating_sub(days);
        for &c in &snap.contributions[start..] {
            let ch = if c > 10 {
                '█'
            } else if c > 5 {
                '▆'
            } else if c > 0 {
                '▃'
            } else {
                '_'
            };
            line.push(ch);
        }
        lines.push(Line::from(Span::styled(
            line,
            Style::default().fg(theme.green),
        )));
    }

    let p = Paragraph::new(lines);
    f.render_widget(p, inner);
}
