use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::app::Theme;

/// Per-core load meters, btop-style. The dashboard's ANALYTICS column only
/// carries aggregate CPU%, which hides one pinned core among idle ones.
pub fn render(f: &mut Frame, area: Rect, theme: &Theme) {
    if area.height < 2 || area.width < 16 {
        return;
    }

    let cores: Vec<f32> = {
        let sys = crate::app::SYS.lock().unwrap();
        sys.cpus().iter().map(|c| c.cpu_usage()).collect()
    };
    if cores.is_empty() {
        return;
    }

    // Two cores per row when there's width for it, else one.
    let per_row = if area.width >= 40 { 2 } else { 1 };
    let rows_needed = cores.len().div_ceil(per_row);
    let rows = rows_needed.min(area.height as usize);

    // Each cell: "c0 " + bar + " 100%"
    let cell_w = area.width as usize / per_row;
    let bar_w = cell_w.saturating_sub(10).max(4);

    // Eighth-block ramp gives sub-cell resolution on short bars.
    const BLOCKS: [&str; 9] = [" ", "▏", "▎", "▍", "▌", "▋", "▊", "▉", "█"];

    let mut lines: Vec<Line> = Vec::with_capacity(rows);
    for r in 0..rows {
        let mut spans: Vec<Span> = Vec::new();
        for c in 0..per_row {
            let idx = r * per_row + c;
            let Some(&usage) = cores.get(idx) else {
                continue;
            };
            let col = if usage >= 90.0 {
                theme.red
            } else if usage >= 70.0 {
                theme.yellow
            } else {
                theme.accent
            };

            let levels = bar_w * 8;
            let filled = ((usage / 100.0).clamp(0.0, 1.0) * levels as f32).round() as usize;
            let mut bar = String::with_capacity(bar_w * 3);
            for i in 0..bar_w {
                bar.push_str(BLOCKS[filled.saturating_sub(i * 8).min(8)]);
            }

            spans.push(Span::styled(
                format!("c{:<2}", idx),
                Style::default().fg(theme.dim),
            ));
            spans.push(Span::styled(bar, Style::default().fg(col)));
            spans.push(Span::styled(
                format!("{:>4.0}% ", usage),
                Style::default().fg(col).add_modifier(Modifier::BOLD),
            ));
        }
        lines.push(Line::from(spans));
    }

    let top = (area.height.saturating_sub(lines.len() as u16)) / 2;
    f.render_widget(
        Paragraph::new(lines),
        Rect::new(area.x, area.y + top, area.width, area.height - top),
    );
}
