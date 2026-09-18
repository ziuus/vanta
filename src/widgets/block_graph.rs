use std::sync::atomic::{AtomicUsize, Ordering};

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::widgets::Widget;

const BLOCKS: [char; 9] = [' ', ' ', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

/// Braille dot bits for a 2×4 cell, top→bottom, left column then right column.
/// (U+2800 base; dots 1237 on the left, 4568 on the right.)
const BRAILLE_LEFT: [u8; 4] = [0x01, 0x02, 0x04, 0x40];
const BRAILLE_RIGHT: [u8; 4] = [0x08, 0x10, 0x20, 0x80];

// ── Style selection ──
// 0 = block (solid mass, any font), 1 = braille (2×4 subpixels, btop-like).
const STYLE_COUNT: usize = 2;
static GRAPH_STYLE: AtomicUsize = AtomicUsize::new(0);

/// Cycle to the next graph style. Bound to `G` globally.
pub fn cycle_style() {
    let next = (GRAPH_STYLE.load(Ordering::Relaxed) + 1) % STYLE_COUNT;
    GRAPH_STYLE.store(next, Ordering::Relaxed);
}

/// Select a style by config name; unknown names fall back to block.
pub fn set_style(name: &str) {
    let idx = match name {
        "braille" => 1,
        _ => 0,
    };
    GRAPH_STYLE.store(idx, Ordering::Relaxed);
}

/// Human label for the active style, for config persistence and the status bar.
pub fn style_name() -> &'static str {
    match GRAPH_STYLE.load(Ordering::Relaxed) % STYLE_COUNT {
        1 => "braille",
        _ => "block",
    }
}

/// A filled area chart, one sample per sub-column, right-aligned so the newest
/// sample sits at the right edge.
///
/// Two render styles (`set_style`): **block** gives up horizontal resolution to
/// render as a solid mass — safe on any terminal font. **braille** packs 2×4
/// subpixels per cell for a finer, btop-style line, but leans on the font
/// rendering braille solidly rather than as a sparse dot-matrix.
pub struct BlockGraph<'a> {
    data: &'a [f64],
    data2: Option<&'a [f64]>,
    min: f64,
    max: f64,
    color_safe: Color,
    color_warn: Color,
    color_crit: Color,
    color2_safe: Option<Color>,
    color2_warn: Option<Color>,
    color2_crit: Option<Color>,
    mirrored: bool,
}

impl<'a> BlockGraph<'a> {
    pub fn new(data: &'a [f64]) -> Self {
        Self {
            data,
            data2: None,
            min: 0.0,
            max: 100.0,
            color_safe: Color::Green,
            color_warn: Color::Yellow,
            color_crit: Color::Red,
            color2_safe: None,
            color2_warn: None,
            color2_crit: None,
            mirrored: false,
        }
    }

    pub fn data2(mut self, data: &'a [f64]) -> Self {
        self.data2 = Some(data);
        self
    }

    pub fn colors2(mut self, safe: Color, warn: Color, crit: Color) -> Self {
        self.color2_safe = Some(safe);
        self.color2_warn = Some(warn);
        self.color2_crit = Some(crit);
        self
    }

    pub fn mirrored(mut self, mirrored: bool) -> Self {
        self.mirrored = mirrored;
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

    /// Value at sub-column `sx` of `sub_w`, right-aligned; None where the
    /// series is shorter than the width and hasn't reached this column yet.
    fn sample_at(&self, sx: usize, sub_w: usize) -> Option<f64> {
        let n = self.data.len();
        if n >= sub_w {
            Some(self.data[n - sub_w + sx])
        } else {
            let start = sub_w - n;
            sx.checked_sub(start).map(|i| self.data[i])
        }
    }

    fn sample_at2(&self, sx: usize, sub_w: usize) -> Option<f64> {
        let d = self.data2?;
        let n = d.len();
        if n >= sub_w {
            Some(d[n - sub_w + sx])
        } else {
            let start = sub_w - n;
            sx.checked_sub(start).map(|i| d[i])
        }
    }

    fn cell_color2(&self, _t: f64, cf: f64) -> Color {
        let safe = self.color2_safe.unwrap_or(self.color_safe);
        let warn = self.color2_warn.unwrap_or(self.color_warn);
        let crit = self.color2_crit.unwrap_or(self.color_crit);
        if cf < 0.5 {
            crate::theme::blend(safe, warn, (cf * 2.0) as f32)
        } else {
            crate::theme::blend(warn, crit, ((cf - 0.5) * 2.0) as f32)
        }
    }

    fn render_blocks(&self, area: Rect, buf: &mut Buffer) {
        let cols = area.width as usize;
        let span = (self.max - self.min).max(f64::EPSILON);
        let steps = area.height as usize * 8;
        let center_y = steps as f64 / 2.0;
        let is_dual = self.data2.is_some();

        for x in 0..cols {
            let v1 = self.sample_at(x, cols).unwrap_or(0.0);
            let t1 = ((v1 - self.min) / span).clamp(0.0, 1.0);
            let lit1 = (t1 * steps as f64).round() as usize;
            let filled_cells1 = (lit1 as f64 / 8.0).max(f64::EPSILON);

            let v2 = self.sample_at2(x, cols).unwrap_or(0.0);
            let t2 = ((v2 - self.min) / span).clamp(0.0, 1.0);
            let lit2 = (t2 * steps as f64).round() as usize;
            let filled_cells2 = (lit2 as f64 / 8.0).max(f64::EPSILON);

            if lit1 == 0 && lit2 == 0 {
                continue;
            }

            for row in 0..area.height as usize {
                let from_bottom = area.height as usize - 1 - row;
                let cell_sub_y_base = from_bottom * 8; // bottom of this cell

                let (ch, color) = if is_dual {
                    let mut is_bottom = false;
                    // top half goes up from center
                    let level = if (cell_sub_y_base as f64) >= center_y {
                        let top_lit = lit1 / 2;
                        let dist_from_center = cell_sub_y_base - (center_y as usize);
                        top_lit.saturating_sub(dist_from_center).min(8)
                    } else {
                        // bottom half goes down from center
                        // cell_sub_y_base + 7 is the top of this cell
                        let bottom_lit = lit2 / 2;
                        let dist_from_center =
                            (center_y as usize).saturating_sub(cell_sub_y_base + 8);
                        is_bottom = true;
                        bottom_lit.saturating_sub(dist_from_center).min(8)
                    };
                    if level == 0 {
                        continue;
                    }

                    let cell_center = (from_bottom as f64 + 0.5) * 8.0;
                    let dist_from_center_cells = (cell_center - center_y).abs() / 8.0;

                    if is_bottom {
                        let cf = dist_from_center_cells / (filled_cells2 / 2.0).max(f64::EPSILON);
                        (BLOCKS[level], self.cell_color2(t2, cf))
                    } else {
                        let cf = dist_from_center_cells / (filled_cells1 / 2.0).max(f64::EPSILON);
                        (BLOCKS[level], self.cell_color(t1, cf))
                    }
                } else if self.mirrored {
                    let half_lit = lit1 / 2;
                    let dist_from_center =
                        ((cell_sub_y_base as f64 + 4.0) - center_y).abs() as usize;
                    let level = half_lit
                        .saturating_sub(dist_from_center.saturating_sub(4))
                        .min(8);
                    if level == 0 {
                        continue;
                    }
                    let cell_center = (from_bottom as f64 + 0.5) * 8.0;
                    let dist_from_center_cells = (cell_center - center_y).abs() / 8.0;
                    let cf = dist_from_center_cells / (filled_cells1 / 2.0).max(f64::EPSILON);
                    (BLOCKS[level], self.cell_color(t1, cf))
                } else {
                    let level = lit1.saturating_sub(from_bottom * 8).min(8);
                    if level == 0 {
                        continue;
                    }
                    let cf = (from_bottom as f64 + 0.5) / filled_cells1;
                    (BLOCKS[level], self.cell_color(t1, cf))
                };

                let (px, py) = (area.left() + x as u16, area.top() + row as u16);
                if let Some(cell) = buf.cell_mut((px, py)) {
                    cell.set_char(ch);
                    cell.set_style(Style::default().fg(color));
                }
            }
        }
    }
    fn render_braille(&self, area: Rect, buf: &mut Buffer) {
        let cols = area.width as usize;
        let sub_w = cols * 2; // two dot-columns per cell
        let sub_h = area.height as usize * 4; // four dot-rows per cell
        let span = (self.max - self.min).max(f64::EPSILON);

        let center_y = sub_h as f64 / 2.0;
        let is_dual = self.data2.is_some();

        let lit: Vec<usize> = (0..sub_w)
            .map(|sx| {
                self.sample_at(sx, sub_w).map_or(0, |v| {
                    let t = ((v - self.min) / span).clamp(0.0, 1.0);
                    (t * sub_h as f64).round() as usize
                })
            })
            .collect();

        let lit2: Vec<usize> = (0..sub_w)
            .map(|sx| {
                self.sample_at2(sx, sub_w).map_or(0, |v| {
                    let t = ((v - self.min) / span).clamp(0.0, 1.0);
                    (t * sub_h as f64).round() as usize
                })
            })
            .collect();

        for cx in 0..cols {
            let (lx, rx) = (cx * 2, cx * 2 + 1);
            let colmax = lit[lx].max(lit[rx]);
            let colmax2 = lit2[lx].max(lit2[rx]);

            if colmax == 0 && colmax2 == 0 {
                continue;
            }

            let t = colmax as f64 / sub_h as f64;
            let filled_cells = (colmax as f64 / 4.0).max(f64::EPSILON);

            let t2 = colmax2 as f64 / sub_h as f64;
            let filled_cells2 = (colmax2 as f64 / 4.0).max(f64::EPSILON);

            for cy in 0..area.height as usize {
                let from_bottom_cell = area.height as usize - 1 - cy;
                let mut bits = 0u8;
                let mut is_bottom_half = false;

                for k in 0..4 {
                    // k=0 is the top dot-row of the cell.
                    let sub_from_bottom = from_bottom_cell * 4 + (3 - k);
                    let sub_y = sub_from_bottom as f64;

                    if is_dual {
                        // Top half (data) goes up from center
                        if sub_y >= center_y {
                            let lx_h = lit[lx] as f64 / 2.0;
                            if sub_y - center_y < lx_h {
                                bits |= BRAILLE_LEFT[k];
                            }
                            let rx_h = lit[rx] as f64 / 2.0;
                            if sub_y - center_y < rx_h {
                                bits |= BRAILLE_RIGHT[k];
                            }
                        } else {
                            // Bottom half (data2) goes down from center
                            let lx_h = lit2[lx] as f64 / 2.0;
                            if center_y - sub_y <= lx_h {
                                bits |= BRAILLE_LEFT[k];
                                is_bottom_half = true;
                            }
                            let rx_h = lit2[rx] as f64 / 2.0;
                            if center_y - sub_y <= rx_h {
                                bits |= BRAILLE_RIGHT[k];
                                is_bottom_half = true;
                            }
                        }
                    } else if self.mirrored {
                        let lx_half = lit[lx] as f64 / 2.0;
                        if (sub_y - center_y).abs() <= lx_half {
                            bits |= BRAILLE_LEFT[k];
                        }
                        let rx_half = lit[rx] as f64 / 2.0;
                        if (sub_y - center_y).abs() <= rx_half {
                            bits |= BRAILLE_RIGHT[k];
                        }
                    } else {
                        if lit[lx] > sub_from_bottom {
                            bits |= BRAILLE_LEFT[k];
                        }
                        if lit[rx] > sub_from_bottom {
                            bits |= BRAILLE_RIGHT[k];
                        }
                    }
                }

                if bits == 0 {
                    continue;
                }

                let cell_center = (from_bottom_cell as f64 + 0.5) * 4.0;

                let (color, ch) = if is_dual {
                    let dist_from_center_cells = (cell_center - center_y).abs() / 4.0;
                    if is_bottom_half {
                        let max_dist = (filled_cells2 / 2.0).max(f64::EPSILON);
                        (
                            self.cell_color2(t2, dist_from_center_cells / max_dist),
                            char::from_u32(0x2800 + bits as u32).unwrap_or(' '),
                        )
                    } else {
                        let max_dist = (filled_cells / 2.0).max(f64::EPSILON);
                        (
                            self.cell_color(t, dist_from_center_cells / max_dist),
                            char::from_u32(0x2800 + bits as u32).unwrap_or(' '),
                        )
                    }
                } else if self.mirrored {
                    let dist_from_center_cells = (cell_center - center_y).abs() / 4.0;
                    let max_dist = (filled_cells / 2.0).max(f64::EPSILON);
                    let cf = dist_from_center_cells / max_dist;
                    (
                        self.cell_color(t, cf),
                        char::from_u32(0x2800 + bits as u32).unwrap_or(' '),
                    )
                } else {
                    let cf = (from_bottom_cell as f64 + 0.5) / filled_cells;
                    (
                        self.cell_color(t, cf),
                        char::from_u32(0x2800 + bits as u32).unwrap_or(' '),
                    )
                };

                let (px, py) = (area.left() + cx as u16, area.top() + cy as u16);
                if let Some(cell) = buf.cell_mut((px, py)) {
                    cell.set_char(ch);
                    cell.set_style(Style::default().fg(color));
                }
            }
        }
    }
}

impl Widget for BlockGraph<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        match GRAPH_STYLE.load(Ordering::Relaxed) % STYLE_COUNT {
            1 => self.render_braille(area, buf),
            _ => self.render_blocks(area, buf),
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
