use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::widgets::Widget;

/// Eighth-block column ramp: one cell resolves 8 vertical steps.
const BLOCKS: [char; 9] = [' ', '▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

/// A filled area chart drawn with block characters, one sample per column.
///
/// Braille packs 2x4 subpixels per cell, but most terminal fonts render it as a
/// visibly sparse dot-matrix — fine for a sparkline, wrong for a filled graph,
/// where it reads as broken rather than solid. Blocks give up horizontal
/// resolution to render as a solid mass.
pub struct BlockGraph<'a> {
    data: &'a [f64],
    min: f64,
    max: f64,
    color_safe: Color,
    color_warn: Color,
    color_crit: Color,
}

impl<'a> BlockGraph<'a> {
    pub fn new(data: &'a [f64]) -> Self {
        Self {
            data,
            min: 0.0,
            max: 100.0,
            color_safe: Color::Green,
            color_warn: Color::Yellow,
            color_crit: Color::Red,
        }
    }

    pub fn max(mut self, max: f64) -> Self {
        self.max = max;
        self
    }

    pub fn colors(mut self, safe: Color, warn: Color, crit: Color) -> Self {
        self.color_safe = safe;
        self.color_warn = warn;
        self.color_crit = crit;
        self
    }

    /// Continuous safe→warn→crit ramp for a magnitude `t`, so the tip of a bar
    /// takes its true heat instead of snapping between three buckets.
    fn ramp(&self, t: f64) -> Color {
        let t = t.clamp(0.0, 1.0) as f32;
        if t < 0.5 {
            crate::theme::blend(self.color_safe, self.color_warn, t * 2.0)
        } else {
            crate::theme::blend(self.color_warn, self.color_crit, (t - 0.5) * 2.0)
        }
    }

    /// Colour for one filled cell: the base of a bar stays the safe colour and
    /// fades up to `ramp(t)` at its tip. `cf` is the cell's fraction of the
    /// bar's own filled height (0 at the base, 1 at the tip) — so a one-row bar
    /// is all tip and still shows a spike's heat, while a tall bar reads as a
    /// gradient with depth.
    fn cell_color(&self, t: f64, cf: f64) -> Color {
        crate::theme::blend(self.color_safe, self.ramp(t), cf.clamp(0.0, 1.0) as f32)
    }
}

impl Widget for BlockGraph<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 {
            return;
        }

        let cols = area.width as usize;
        let span = (self.max - self.min).max(f64::EPSILON);
        // Total vertical resolution: 8 sub-steps per row.
        let steps = area.height as usize * 8;

        for x in 0..cols {
            // Right-align: the newest sample sits at the right edge, and a
            // partly-filled series leaves the left blank rather than stretching.
            let n = self.data.len();
            let filled = if n >= cols {
                // More data than columns: take the most recent `cols` samples.
                let idx = n - cols + x;
                Some(self.data[idx])
            } else {
                let start = cols - n;
                x.checked_sub(start).map(|i| self.data[i])
            };

            let Some(v) = filled else { continue };
            let t = ((v - self.min) / span).clamp(0.0, 1.0);
            let lit = (t * steps as f64).round() as usize;
            // Filled height of this bar, in fractional cells, for the gradient.
            let filled_cells = (lit as f64 / 8.0).max(f64::EPSILON);

            for row in 0..area.height as usize {
                // Row 0 is the top; fill grows from the bottom up.
                let from_bottom = area.height as usize - 1 - row;
                let level = lit.saturating_sub(from_bottom * 8).min(8);
                if level == 0 {
                    continue;
                }
                // This cell's height up the bar, 0 at the base and 1 at the tip.
                let cf = (from_bottom as f64 + 0.5) / filled_cells;
                let color = self.cell_color(t, cf);
                let (px, py) = (area.left() + x as u16, area.top() + row as u16);
                if let Some(cell) = buf.cell_mut((px, py)) {
                    cell.set_char(BLOCKS[level]);
                    cell.set_style(Style::default().fg(color));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::style::Color;

    fn g() -> BlockGraph<'static> {
        BlockGraph::new(&[]).colors(
            Color::Rgb(0, 255, 0),
            Color::Rgb(255, 255, 0),
            Color::Rgb(255, 0, 0),
        )
    }

    #[test]
    fn ramp_hits_the_three_stops() {
        let g = g();
        assert_eq!(g.ramp(0.0), Color::Rgb(0, 255, 0));
        assert_eq!(g.ramp(0.5), Color::Rgb(255, 255, 0));
        assert_eq!(g.ramp(1.0), Color::Rgb(255, 0, 0));
    }

    /// The one-row graph must still show a spike's heat: at the tip (cf≈1) a
    /// hot column is the crit colour, not the safe base. This is the property
    /// the gradient could regress if it coloured purely by vertical position.
    #[test]
    fn tip_of_a_hot_bar_is_hot() {
        let g = g();
        assert_eq!(g.cell_color(1.0, 1.0), Color::Rgb(255, 0, 0));
        // Base of the same bar is the safe colour — that's the gradient.
        assert_eq!(g.cell_color(1.0, 0.0), Color::Rgb(0, 255, 0));
    }
}
