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

    pub fn min(mut self, min: f64) -> Self {
        self.min = min;
        self
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

    /// Colour by magnitude, so a spike is visible even in a one-row graph.
    fn color_for(&self, t: f64) -> Color {
        if t >= 0.85 {
            self.color_crit
        } else if t >= 0.6 {
            self.color_warn
        } else {
            self.color_safe
        }
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
            let color = self.color_for(t);

            for row in 0..area.height as usize {
                // Row 0 is the top; fill grows from the bottom up.
                let from_bottom = area.height as usize - 1 - row;
                let level = lit.saturating_sub(from_bottom * 8).min(8);
                if level == 0 {
                    continue;
                }
                let (px, py) = (area.left() + x as u16, area.top() + row as u16);
                if let Some(cell) = buf.cell_mut((px, py)) {
                    cell.set_char(BLOCKS[level]);
                    cell.set_style(Style::default().fg(color));
                }
            }
        }
    }
}
