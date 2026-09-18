import sys

content = open('src/widgets/block_graph.rs').read()

import re

# We need to add sample_at2
search_sample = """    fn sample_at(&self, sx: usize, sub_w: usize) -> Option<f64> {
        let len = self.data.len();
        if len == 0 {
            return None;
        }
        let offset = sub_w.saturating_sub(len);
        if sx < offset {
            return None;
        }
        let idx = (sx - offset).min(len - 1);
        Some(self.data[idx])
    }"""

replacement_sample = search_sample + """

    fn sample_at2(&self, sx: usize, sub_w: usize) -> Option<f64> {
        let d = self.data2?;
        let len = d.len();
        if len == 0 {
            return None;
        }
        let offset = sub_w.saturating_sub(len);
        if sx < offset {
            return None;
        }
        let idx = (sx - offset).min(len - 1);
        Some(d[idx])
    }

    fn cell_color2(&self, _t: f64, cf: f64) -> Color {
        let safe = self.color2_safe.unwrap_or(self.color_safe);
        let warn = self.color2_warn.unwrap_or(self.color_warn);
        let crit = self.color2_crit.unwrap_or(self.color_crit);
        if cf < 0.5 {
            blend(safe, warn, (cf * 2.0) as f32)
        } else {
            blend(warn, crit, ((cf - 0.5) * 2.0) as f32)
        }
    }"""

content = content.replace(search_sample, replacement_sample)

# Rewrite render_braille completely
start = content.find("fn render_braille")
end = content.find("    }\n}", start) + 5

new_braille = """    fn render_braille(&self, area: Rect, buf: &mut Buffer) {
        let cols = area.width as usize;
        let sub_w = cols * 2; // two dot-columns per cell
        let sub_h = area.height as usize * 4; // four dot-rows per cell
        let span = (self.max - self.min).max(f64::EPSILON);
        
        let center_y = sub_h as f64 / 2.0;
        let is_dual = self.data2.is_some();

        let lit: Vec<usize> = (0..sub_w).map(|sx| {
            self.sample_at(sx, sub_w).map_or(0, |v| {
                let t = ((v - self.min) / span).clamp(0.0, 1.0);
                (t * sub_h as f64).round() as usize
            })
        }).collect();
        
        let lit2: Vec<usize> = (0..sub_w).map(|sx| {
            self.sample_at2(sx, sub_w).map_or(0, |v| {
                let t = ((v - self.min) / span).clamp(0.0, 1.0);
                (t * sub_h as f64).round() as usize
            })
        }).collect();

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
                            if sub_y - center_y < lx_h { bits |= BRAILLE_LEFT[k]; }
                            let rx_h = lit[rx] as f64 / 2.0;
                            if sub_y - center_y < rx_h { bits |= BRAILLE_RIGHT[k]; }
                        } else {
                            // Bottom half (data2) goes down from center
                            let lx_h = lit2[lx] as f64 / 2.0;
                            if center_y - sub_y <= lx_h { bits |= BRAILLE_LEFT[k]; is_bottom_half = true; }
                            let rx_h = lit2[rx] as f64 / 2.0;
                            if center_y - sub_y <= rx_h { bits |= BRAILLE_RIGHT[k]; is_bottom_half = true; }
                        }
                    } else if self.mirrored {
                        let lx_half = lit[lx] as f64 / 2.0;
                        if (sub_y - center_y).abs() <= lx_half { bits |= BRAILLE_LEFT[k]; }
                        let rx_half = lit[rx] as f64 / 2.0;
                        if (sub_y - center_y).abs() <= rx_half { bits |= BRAILLE_RIGHT[k]; }
                    } else {
                        if lit[lx] > sub_from_bottom { bits |= BRAILLE_LEFT[k]; }
                        if lit[rx] > sub_from_bottom { bits |= BRAILLE_RIGHT[k]; }
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
                        (self.cell_color2(t2, dist_from_center_cells / max_dist), char::from_u32(0x2800 + bits as u32).unwrap_or(' '))
                    } else {
                        let max_dist = (filled_cells / 2.0).max(f64::EPSILON);
                        (self.cell_color(t, dist_from_center_cells / max_dist), char::from_u32(0x2800 + bits as u32).unwrap_or(' '))
                    }
                } else if self.mirrored {
                    let dist_from_center_cells = (cell_center - center_y).abs() / 4.0;
                    let max_dist = (filled_cells / 2.0).max(f64::EPSILON);
                    let cf = dist_from_center_cells / max_dist;
                    (self.cell_color(t, cf), char::from_u32(0x2800 + bits as u32).unwrap_or(' '))
                } else {
                    let cf = (from_bottom_cell as f64 + 0.5) / filled_cells;
                    (self.cell_color(t, cf), char::from_u32(0x2800 + bits as u32).unwrap_or(' '))
                };
                
                let (px, py) = (area.left() + cx as u16, area.top() + cy as u16);
                if let Some(cell) = buf.cell_mut((px, py)) {
                    cell.set_char(ch);
                    cell.set_style(Style::default().fg(color));
                }
            }
        }
    }"""

content = content[:start] + new_braille + content[end:]
open('src/widgets/block_graph.rs', 'w').write(content)
print("Rewrote render_braille")
