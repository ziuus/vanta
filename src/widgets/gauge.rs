use std::f64::consts::PI;

use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::app::Theme;

/// Gauge box in cells. Cells are roughly twice as tall as they are wide, so a
/// 2:1 grid is what makes the ellipse read as a circle on screen.
const W: usize = 15;
const H: usize = 8;

/// The arc spans 180 degrees (a semicircle).
const SWEEP: f64 = 180.0;
/// Ring thickness as a fraction of the outer radius.
const INNER: f64 = 0.55;

/// One ring: solid blocks for the filled arc, a dim track for the remainder,
/// label and value stacked in the hollow centre.
///
/// Uses full blocks rather than braille because braille renders as a visibly
/// sparse dot-matrix in most terminal fonts, which reads as broken rather than
/// as a gauge.
#[derive(Clone, Copy, PartialEq)]
enum Pxl {
    Empty,
    Filled,
    Dim,
}

#[allow(clippy::needless_range_loop)]
fn ring(pct: f64, label: &str, value: &str, col: Color, theme: &Theme) -> Vec<Line<'static>> {
    let sweep = (pct.clamp(0.0, 100.0) / 100.0) * SWEEP;

    let h_px = H * 2;
    let mut pixels = vec![vec![Pxl::Empty; W]; h_px];

    for py in 0..h_px {
        for px in 0..W {
            let dx = (px as f64 - W as f64 / 2.0 + 0.5) / (W as f64 / 2.0);
            let dy = (py as f64 - h_px as f64 / 2.0 + 0.5) / (h_px as f64 / 2.0);
            let r = (dx * dx + dy * dy).sqrt();

            if !(INNER..=1.02).contains(&r) {
                continue;
            }

            // -180.0 starts the arc at the left (9 o'clock)
            let ang = (dy.atan2(dx) * 180.0 / PI - 180.0).rem_euclid(360.0);
            if ang > SWEEP {
                pixels[py][px] = Pxl::Empty;
            } else if ang <= sweep {
                pixels[py][px] = Pxl::Filled;
            } else {
                pixels[py][px] = Pxl::Dim;
            }
        }
    }

    let mut rows = Vec::new();
    for y in 0..H {
        let mut spans = Vec::new();
        for x in 0..W {
            let top = pixels[y * 2][x];
            let bot = pixels[y * 2 + 1][x];

            let span = match (top, bot) {
                (Pxl::Empty, Pxl::Empty) => Span::raw(" "),
                (Pxl::Filled, Pxl::Filled) => Span::styled("█", Style::default().fg(col)),
                (Pxl::Dim, Pxl::Dim) => Span::styled("█", Style::default().fg(theme.surface)),
                (Pxl::Filled, Pxl::Empty) => Span::styled("▀", Style::default().fg(col)),
                (Pxl::Empty, Pxl::Filled) => Span::styled("▄", Style::default().fg(col)),
                (Pxl::Dim, Pxl::Empty) => Span::styled("▀", Style::default().fg(theme.surface)),
                (Pxl::Empty, Pxl::Dim) => Span::styled("▄", Style::default().fg(theme.surface)),
                (Pxl::Filled, Pxl::Dim) => {
                    Span::styled("▀", Style::default().fg(col).bg(theme.surface))
                }
                (Pxl::Dim, Pxl::Filled) => {
                    Span::styled("▄", Style::default().fg(col).bg(theme.surface))
                }
            };
            spans.push(span);
        }
        rows.push(Line::from(spans));
    }

    // Place label and value in the hollow centre, slightly lower so it's under the arch
    let centre = |text: &str, style: Style| -> Line<'static> {
        let n = text.chars().count();
        let pad = W.saturating_sub(n) / 2;
        Line::from(vec![
            Span::raw(" ".repeat(pad)),
            Span::styled(text.to_string(), style),
            Span::raw(" ".repeat(W.saturating_sub(pad + n))),
        ])
    };
    if H >= 6 {
        rows[4] = centre(label, Style::default().fg(theme.dim));
        rows[5] = centre(value, Style::default().fg(col).add_modifier(Modifier::BOLD));
    }
    rows
}

/// Render up to three circular gauges side by side.
///
/// Each entry is `(label, percent, display value, colour)`: the percent drives
/// the arc while the display value is what sits in the middle, so a ring can
/// read "6.2G" while the arc tracks 83%.
pub fn render(f: &mut Frame, area: Rect, theme: &Theme, metrics: &[(&str, f64, String, Color)]) {
    if area.height < H as u16 || metrics.is_empty() {
        return;
    }

    let gap = 1usize;
    let n = metrics
        .len()
        .min(3)
        .min(((area.width as usize + gap) / (W + gap)).max(1));
    let total = n * W + n.saturating_sub(1) * gap;
    let pad = (area.width as usize).saturating_sub(total) / 2;

    let mut rows: Vec<Line<'static>> = (0..H)
        .map(|_| Line::from(vec![Span::raw(" ".repeat(pad))]))
        .collect();

    for (i, (label, pct, value, col)) in metrics.iter().take(n).enumerate() {
        for (r, line) in ring(*pct, label, value, *col, theme)
            .into_iter()
            .enumerate()
        {
            if i > 0 {
                rows[r].spans.push(Span::raw(" ".repeat(gap)));
            }
            rows[r].spans.extend(line.spans);
        }
    }

    let top = (area.height.saturating_sub(H as u16)) / 2;
    f.render_widget(
        Paragraph::new(rows),
        Rect::new(area.x, area.y + top, area.width, H as u16),
    );
}
