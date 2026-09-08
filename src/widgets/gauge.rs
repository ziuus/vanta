use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::theme::Theme;

/// One gauge is W cells wide and H rows tall. Cells are ~1:2, so a W/2 == 2H
/// grid renders the semicircle as a true half-circle on screen.
pub const W: usize = 16;
pub const H: usize = 4;

/// Ring thickness as a fraction of the outer radius.
const INNER: f64 = 0.58;

#[derive(Clone, Copy, PartialEq)]
enum Px {
    Empty,
    Fill,
    Track,
}

/// A 180° arc, centre at the bottom-middle, drawn with half-block pixels so
/// the ring is solid. Label and value sit in the hollow under the arch.
fn ring(pct: f64, label: &str, value: &str, col: Color, theme: &Theme) -> Vec<Line<'static>> {
    let sweep = pct.clamp(0.0, 100.0) / 100.0 * 180.0;
    let h_px = H * 2;
    let mut px = vec![vec![Px::Empty; W]; h_px];

    for (y, row) in px.iter_mut().enumerate() {
        for (x, cell) in row.iter_mut().enumerate() {
            // Centre is at the bottom edge; radius = W/2 horizontally, h_px vertically.
            let dx = (x as f64 + 0.5 - W as f64 / 2.0) / (W as f64 / 2.0);
            let dy = (h_px as f64 - (y as f64 + 0.5)) / h_px as f64;
            let r = (dx * dx + dy * dy).sqrt();
            if !(INNER..=1.0).contains(&r) {
                continue;
            }
            // 0° at the left (9 o'clock) sweeping clockwise over the top to 180°.
            let ang = (-dy).atan2(dx).to_degrees(); // -180..0 on the top half
            let ang = ang + 180.0; // 0 (left) .. 180 (right)
            *cell = if ang <= sweep { Px::Fill } else { Px::Track };
        }
    }

    let fill = Style::default().fg(col);
    let track = Style::default().fg(theme.surface);
    let mut rows: Vec<Line<'static>> = (0..H)
        .map(|y| {
            let spans = (0..W)
                .map(|x| match (px[y * 2][x], px[y * 2 + 1][x]) {
                    (Px::Empty, Px::Empty) => Span::raw(" "),
                    (Px::Fill, Px::Fill) => Span::styled("█", fill),
                    (Px::Track, Px::Track) => Span::styled("█", track),
                    (Px::Fill, Px::Empty) => Span::styled("▀", fill),
                    (Px::Empty, Px::Fill) => Span::styled("▄", fill),
                    (Px::Track, Px::Empty) => Span::styled("▀", track),
                    (Px::Empty, Px::Track) => Span::styled("▄", track),
                    (Px::Fill, Px::Track) => Span::styled("▀", fill.bg(theme.surface)),
                    (Px::Track, Px::Fill) => Span::styled("▄", fill.bg(theme.surface)),
                })
                .collect::<Vec<_>>();
            Line::from(spans)
        })
        .collect();

    // Overwrite the hollow centre of the bottom two rows with text.
    let hollow = ((W as f64 / 2.0) * INNER * 2.0) as usize - 2; // usable cells
    let start = (W - hollow) / 2;
    let put = |line: &mut Line<'static>, text: &str, style: Style| {
        let n = text.chars().count().min(hollow);
        let pad_l = start + (hollow - n) / 2;
        let text: String = text.chars().take(n).collect();
        let mut spans: Vec<Span<'static>> = line.spans.drain(..pad_l).collect();
        spans.push(Span::styled(text, style));
        spans.extend(line.spans.drain(n..));
        line.spans = spans;
    };
    put(&mut rows[H - 2], label, Style::default().fg(theme.dim));
    put(
        &mut rows[H - 1],
        value,
        Style::default().fg(col).add_modifier(Modifier::BOLD),
    );
    rows
}

/// Render as many gauges as fit side by side, centred. Each entry is
/// `(label, percent, display value, colour)`.
pub fn render(f: &mut Frame, area: Rect, theme: &Theme, metrics: &[(&str, f64, String, Color)]) {
    if area.height < H as u16 || metrics.is_empty() {
        return;
    }
    let gap = 2usize;
    let n = metrics
        .len()
        .min(((area.width as usize + gap) / (W + gap)).max(1));
    let total = n * W + (n - 1) * gap;
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
    let top = area.height.saturating_sub(H as u16) / 2;
    f.render_widget(
        Paragraph::new(rows),
        Rect::new(area.x, area.y + top, area.width, H as u16),
    );
}
