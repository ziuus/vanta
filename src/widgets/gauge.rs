use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::app::Theme;

/// A gauge ring is drawn in a 13x7 cell box.
const W: usize = 13;
const H: usize = 7;

/// Braille dot bit for a (row, col) position within a 2x4 cell.
fn dot_bit(dy: usize, dx: usize) -> Option<u8> {
    Some(match (dy, dx) {
        (0, 0) => 0,
        (1, 0) => 1,
        (2, 0) => 2,
        (3, 0) => 6,
        (0, 1) => 3,
        (1, 1) => 4,
        (2, 1) => 5,
        (3, 1) => 7,
        _ => return None,
    })
}

/// Rasterise a 270-degree arc into a braille grid.
///
/// Terminal cells are 2 dots wide but 4 dots tall, so the vertical radius is
/// twice the horizontal one — otherwise the "circle" renders as a squashed oval.
fn arc_grid(pct: f64) -> Vec<Vec<u8>> {
    let filled = (pct.clamp(0.0, 100.0) / 100.0) * 270.0;
    let (cx, cy) = (W as f64, (H * 4) as f64 / 2.0);
    let (rx_o, ry_o) = (10.0, 20.0);
    let (rx_i, ry_i) = (6.5, 13.0);

    let mut grid = vec![vec![0u8; W]; H];
    let put = |x: f64, y: f64, grid: &mut Vec<Vec<u8>>| {
        if x < 0.0 || y < 0.0 {
            return;
        }
        let (cell_x, cell_y) = ((x / 2.0) as usize, (y / 4.0) as usize);
        if cell_x >= W || cell_y >= H {
            return;
        }
        if let Some(bit) = dot_bit(y as usize % 4, x as usize % 2) {
            grid[cell_y][cell_x] |= 1 << bit;
        }
    };

    // Quarter-degree steps keep the arc solid with no gaps at the outer edge.
    let steps = (filled * 4.0) as usize;
    for i in 0..=steps {
        let deg = i as f64 / 4.0;
        if deg > filled {
            break;
        }
        // Start at bottom-left (135 deg) and sweep clockwise.
        let (sin, cos) = (deg + 135.0).to_radians().sin_cos();
        for k in 0..=8 {
            let t = k as f64 / 8.0;
            let rx = rx_i + (rx_o - rx_i) * t;
            let ry = ry_i + (ry_o - ry_i) * t;
            put(cx + cos * rx, cy + sin * ry, &mut grid);
        }
    }
    grid
}

/// One ring with its label and value stacked in the hollow centre.
fn render_ring(pct: f64, label: &str, value: &str, col: Color, theme: &Theme) -> Vec<Line<'static>> {
    let grid = arc_grid(pct);

    let mut lines: Vec<Line<'static>> = grid
        .iter()
        .map(|row| {
            Line::from(
                row.iter()
                    .map(|&p| {
                        let ch = if p == 0 {
                            ' '
                        } else {
                            char::from_u32(0x2800 + p as u32).unwrap_or(' ')
                        };
                        Span::styled(ch.to_string(), Style::default().fg(col))
                    })
                    .collect::<Vec<_>>(),
            )
        })
        .collect();

    // Punch the label and value into the middle rows, keeping the ring around them.
    let centre = |text: &str, style: Style| -> Line<'static> {
        let pad = W.saturating_sub(text.chars().count()) / 2;
        Line::from(vec![
            Span::raw(" ".repeat(pad)),
            Span::styled(text.to_string(), style),
            Span::raw(" ".repeat(W.saturating_sub(pad + text.chars().count()))),
        ])
    };
    if lines.len() >= 5 {
        lines[2] = centre(label, Style::default().fg(theme.dim));
        lines[3] = centre(
            value,
            Style::default().fg(col).add_modifier(Modifier::BOLD),
        );
    }
    lines
}

/// Render up to three circular gauges side by side.
///
/// Each entry is `(label, percent, display value, colour)` — percent drives the
/// arc, the display value is what goes in the middle (so a gauge can show
/// "6.2G" while the arc tracks 83%).
pub fn render(f: &mut Frame, area: Rect, theme: &Theme, metrics: &[(&str, f64, String, Color)]) {
    if area.height < H as u16 || metrics.is_empty() {
        return;
    }

    let gap = 2usize;
    // Fit as many rings as the panel is wide enough to hold.
    let n = metrics
        .len()
        .min(3)
        .min(((area.width as usize + gap) / (W + gap)).max(1));
    let total = n * W + (n - 1) * gap;
    let x_pad = (area.width as usize).saturating_sub(total) / 2;

    let mut rows: Vec<Line<'static>> = vec![Line::from(vec![Span::raw(" ".repeat(x_pad))]); H];
    for (i, (label, pct, value, col)) in metrics.iter().take(n).enumerate() {
        let ring = render_ring(*pct, label, value, *col, theme);
        for (r, line) in ring.into_iter().enumerate() {
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
