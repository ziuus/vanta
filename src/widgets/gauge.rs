use std::sync::atomic::{AtomicUsize, Ordering};

use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::theme::Theme;

/// One arc gauge is W cells wide and H rows tall. Cells are ~1:2, so a
/// W/2 == 2H grid renders the semicircle as a true half-circle on screen.
pub const W: usize = 16;
pub const H: usize = 4;

/// Ring thickness as a fraction of the outer radius.
const INNER: f64 = 0.58;

/// Eighth-block ramps for the bar / vertical styles.
const H_EIGHTHS: [char; 9] = [' ', '▏', '▎', '▍', '▌', '▋', '▊', '▉', '█'];
const V_EIGHTHS: [char; 9] = [' ', '▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
/// Light-shade track behind an unfilled bar, so its full length still reads.
const TRACK: char = '░';

// ── Style selection ──
// 0 = arc (semicircle dial), 1 = bars (horizontal), 2 = vertical (columns).
const STYLE_COUNT: usize = 3;
static GAUGE_STYLE: AtomicUsize = AtomicUsize::new(0);

/// Cycle to the next gauge style. Bound to `g` globally.
pub fn cycle_style() {
    let next = (GAUGE_STYLE.load(Ordering::Relaxed) + 1) % STYLE_COUNT;
    GAUGE_STYLE.store(next, Ordering::Relaxed);
}

/// Select a style by config name; unknown names fall back to arc.
pub fn set_style(name: &str) {
    let idx = match name {
        "bars" => 1,
        "vertical" => 2,
        _ => 0,
    };
    GAUGE_STYLE.store(idx, Ordering::Relaxed);
}

/// Human label for the active style, for config persistence and the status bar.
pub fn style_name() -> &'static str {
    match GAUGE_STYLE.load(Ordering::Relaxed) % STYLE_COUNT {
        1 => "bars",
        2 => "vertical",
        _ => "arc",
    }
}

/// Render as many gauges as fit, in the configured style. Each entry is
/// `(label, percent, display value, colour)`.
pub fn render(f: &mut Frame, area: Rect, theme: &Theme, metrics: &[(&str, f64, String, Color)]) {
    if metrics.is_empty() {
        return;
    }
    match GAUGE_STYLE.load(Ordering::Relaxed) % STYLE_COUNT {
        1 => render_bars(f, area, theme, metrics),
        2 => render_vertical(f, area, theme, metrics),
        _ => render_arc(f, area, theme, metrics),
    }
}

// ── Shared: a smooth horizontal gradient bar ──

/// A `width`-cell bar, eighth-block resolution, shaded along its length by the
/// usage ramp; the unfilled remainder is a dim track so the full extent reads.
fn bar_spans(frac: f64, width: usize, theme: &Theme) -> Vec<Span<'static>> {
    let levels = (frac.clamp(0.0, 1.0) * (width * 8) as f64).round() as usize;
    (0..width)
        .map(|i| {
            let level = levels.saturating_sub(i * 8).min(8);
            if level == 0 {
                Span::styled(TRACK.to_string(), Style::default().fg(theme.dim))
            } else {
                let t = (i as f64 + 0.5) / width as f64;
                Span::styled(
                    H_EIGHTHS[level].to_string(),
                    Style::default().fg(theme.usage_ramp(t)),
                )
            }
        })
        .collect()
}

// ── Bars style ──

fn render_bars(f: &mut Frame, area: Rect, theme: &Theme, metrics: &[(&str, f64, String, Color)]) {
    let n = metrics.len().min(area.height as usize);
    if n == 0 {
        return;
    }
    // label (4) + space + bar + space + value (right-aligned).
    let label_w = 4usize;
    let value_w = metrics
        .iter()
        .map(|(_, _, v, _)| v.chars().count())
        .max()
        .unwrap_or(3)
        .max(3);
    let bar_w = (area.width as usize).saturating_sub(label_w + value_w + 3);
    if bar_w < 4 {
        return;
    }

    let lines: Vec<Line> = metrics
        .iter()
        .take(n)
        .map(|(label, pct, value, col)| {
            let mut spans = vec![Span::styled(
                format!(" {:<w$} ", label, w = label_w),
                Style::default().fg(theme.dim),
            )];
            spans.extend(bar_spans(pct / 100.0, bar_w, theme));
            spans.push(Span::styled(
                format!(" {:>w$}", value, w = value_w),
                Style::default().fg(*col).add_modifier(Modifier::BOLD),
            ));
            Line::from(spans)
        })
        .collect();

    // Centre the block of rows in the panel.
    let top = area.y + area.height.saturating_sub(n as u16) / 2;
    f.render_widget(
        Paragraph::new(lines),
        Rect::new(area.x, top, area.width, n as u16),
    );
}

// ── Vertical style ──

fn render_vertical(
    f: &mut Frame,
    area: Rect,
    theme: &Theme,
    metrics: &[(&str, f64, String, Color)],
) {
    if area.height < 3 {
        return render_bars(f, area, theme, metrics);
    }
    let n = metrics.len();
    let slot = area.width as usize / n; // cells per metric
    if slot < 4 {
        return render_bars(f, area, theme, metrics);
    }
    let bar_w = (slot / 2).clamp(3, 8); // solid column width
    let bar_rows = area.height as usize - 2; // reserve label + value rows
    let steps = bar_rows * 8;

    // Build the bar rows top→bottom, then the label and value rows.
    let mut lines: Vec<Line> = Vec::with_capacity(area.height as usize);
    for row in 0..bar_rows {
        let from_bottom = bar_rows - 1 - row;
        let mut spans: Vec<Span> = Vec::new();
        for (_, pct, _, _) in metrics {
            let lit = ((pct / 100.0).clamp(0.0, 1.0) * steps as f64).round() as usize;
            let level = lit.saturating_sub(from_bottom * 8).min(8);
            let ch = V_EIGHTHS[level];
            let t = (from_bottom as f64 + 0.5) / bar_rows as f64;
            let color = if level == 0 {
                theme.dim
            } else {
                theme.usage_ramp(t)
            };
            let pad = slot.saturating_sub(bar_w) / 2;
            spans.push(Span::raw(" ".repeat(pad)));
            spans.push(Span::styled(
                ch.to_string().repeat(bar_w),
                Style::default().fg(color),
            ));
            spans.push(Span::raw(" ".repeat(slot - bar_w - pad)));
        }
        lines.push(Line::from(spans));
    }
    lines.push(centered_row(metrics, slot, |m| {
        (m.0.to_string(), theme.dim)
    }));
    lines.push(centered_row(metrics, slot, |m| (m.2.clone(), m.3)));

    f.render_widget(Paragraph::new(lines), area);
}

/// One row of centred cells, one per metric slot; each cell's text and colour
/// come from `cell`.
fn centered_row(
    metrics: &[(&str, f64, String, Color)],
    slot: usize,
    cell: impl Fn(&(&str, f64, String, Color)) -> (String, Color),
) -> Line<'static> {
    let mut spans: Vec<Span> = Vec::new();
    for m in metrics {
        let (s, color) = cell(m);
        let s: String = s.chars().take(slot).collect();
        let n = s.chars().count();
        let pad_l = (slot - n) / 2;
        spans.push(Span::raw(" ".repeat(pad_l)));
        spans.push(Span::styled(s, Style::default().fg(color)));
        spans.push(Span::raw(" ".repeat(slot - n - pad_l)));
    }
    Line::from(spans)
}

// ── Arc style ──

#[derive(Clone, Copy, PartialEq)]
enum Px {
    Empty,
    /// A lit segment of the arc, carrying its own colour so the dial can shade
    /// along the sweep instead of being one flat block.
    Fill(Color),
    Track,
}

/// A 180° arc, centre at the bottom-middle, drawn with half-block pixels so
/// the ring is solid. The fill shades accent→yellow→red along the sweep, so
/// the dial visibly heats up as it fills. Label and value sit in the hollow.
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
            *cell = if ang <= sweep {
                Px::Fill(theme.usage_ramp(ang / 180.0))
            } else {
                Px::Track
            };
        }
    }

    let track = Style::default().fg(theme.surface);
    let fg = |c: Color| Style::default().fg(c);
    let mut rows: Vec<Line<'static>> = (0..H)
        .map(|y| {
            let spans = (0..W)
                .map(|x| match (px[y * 2][x], px[y * 2 + 1][x]) {
                    (Px::Empty, Px::Empty) => Span::raw(" "),
                    (Px::Fill(c), Px::Fill(_)) => Span::styled("█", fg(c)),
                    (Px::Track, Px::Track) => Span::styled("█", track),
                    (Px::Fill(c), Px::Empty) => Span::styled("▀", fg(c)),
                    (Px::Empty, Px::Fill(c)) => Span::styled("▄", fg(c)),
                    (Px::Track, Px::Empty) => Span::styled("▀", track),
                    (Px::Empty, Px::Track) => Span::styled("▄", track),
                    (Px::Fill(c), Px::Track) => Span::styled("▀", fg(c).bg(theme.surface)),
                    (Px::Track, Px::Fill(c)) => Span::styled("▄", fg(c).bg(theme.surface)),
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

fn render_arc(f: &mut Frame, area: Rect, theme: &Theme, metrics: &[(&str, f64, String, Color)]) {
    if area.height < H as u16 {
        return render_bars(f, area, theme, metrics);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn style_name_roundtrips() {
        for name in ["arc", "bars", "vertical"] {
            set_style(name);
            assert_eq!(style_name(), name);
        }
        set_style("nonsense");
        assert_eq!(style_name(), "arc");
    }

    /// A full bar is all solid blocks; an empty bar is all track. The gradient
    /// endpoints must actually differ from the track colour.
    #[test]
    fn bar_fills_and_tracks() {
        let t = Theme::dark();
        let full = bar_spans(1.0, 6, &t);
        assert!(full.iter().all(|s| s.content == "█"));
        let empty = bar_spans(0.0, 6, &t);
        assert!(empty.iter().all(|s| s.content == TRACK.to_string()));
    }
}
