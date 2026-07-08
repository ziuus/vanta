use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::app;

/// Animated spectrum-analyzer-style bars (purely visual, no actual audio input yet).
/// Uses a deterministic pseudo-random pattern based on tick count.
pub fn render(f: &mut Frame, area: Rect, theme: &app::Theme, tick: u64) {
    let cols = area.width as usize;
    let height = area.height as usize;
    if cols < 4 || height < 3 {
        return;
    }

    let bar_count = cols.saturating_sub(2).min(32);
    let max_row = (height.saturating_sub(1)) as u16;

    // Generate bar heights using deterministic "melody"
    let seed = tick / 6;
    let heights: Vec<u16> = (0..bar_count)
        .map(|i| pseudo_sin(seed.wrapping_add(i as u64 * 7), max_row))
        .collect();

    // Normalize to fit height
    let max_val = heights.iter().max().copied().unwrap_or(1).max(1);
    let heights: Vec<u16> = heights
        .iter()
        .map(|h| *h * max_row / max_val)
        .collect();

    // Build rows from bottom to top
    let mut ratatui_lines = Vec::with_capacity(height);

    for row in (0..height).rev() {
        let mut line_str = String::with_capacity(cols);
        let row_h = row as u16;
        for h in &heights {
            if *h > row_h {
                if row_h == max_row {
                    line_str.push('▄');
                } else if *h - row_h >= 2 {
                    line_str.push('█');
                } else {
                    line_str.push('▄');
                }
            } else {
                line_str.push(' ');
            }
        }
        ratatui_lines.push(Line::from(Span::styled(
            line_str,
            Style::default().fg(theme.accent),
        )));
    }

    f.render_widget(
        Paragraph::new(ratatui_lines).style(Style::default().bg(theme.surface)),
        area,
    );
}

/// Simple pseudo-sine approximation using integer arithmetic
fn pseudo_sin(t: u64, max: u16) -> u16 {
    let max = max as f64;
    let t = t as f64 * 0.3;
    let v = (t.sin() + (t * 1.7 + 1.2).sin() * 0.6 + (t * 3.2 + 4.8).sin() * 0.3) * 0.5 + 0.5;
    (v * max) as u16
}
