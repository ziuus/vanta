use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::widgets::Widget;

pub struct BrailleGraph<'a> {
    data: &'a [f64],
    min: f64,
    max: f64,
    color_safe: Color,
    color_warn: Color,
    color_crit: Color,
}

impl<'a> BrailleGraph<'a> {
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
}

fn lerp_color(a: Color, b: Color, t: f64) -> Color {
    let (ar, ag, ab) = match a {
        Color::Rgb(r, g, b) => (r, g, b),
        _ => (0, 255, 0),
    };
    let (br, bg, bb) = match b {
        Color::Rgb(r, g, b) => (r, g, b),
        _ => (255, 0, 0),
    };

    Color::Rgb(
        (ar as f64 + (br as f64 - ar as f64) * t) as u8,
        (ag as f64 + (bg as f64 - ag as f64) * t) as u8,
        (ab as f64 + (bb as f64 - ab as f64) * t) as u8,
    )
}

fn gradient_color(value: f64, min: f64, max: f64, safe: Color, warn: Color, crit: Color) -> Color {
    let t = ((value - min) / (max - min).max(f64::EPSILON)).clamp(0.0, 1.0);
    if t < 0.6 {
        lerp_color(safe, safe, t / 0.6)
    } else if t < 0.85 {
        lerp_color(safe, warn, (t - 0.6) / 0.25)
    } else {
        lerp_color(warn, crit, (t - 0.85) / 0.15)
    }
}

fn normalize(value: f64, min: f64, max: f64, dot_rows: usize) -> usize {
    let clamped = value.clamp(min, max);
    let ratio = (clamped - min) / (max - min).max(f64::EPSILON);
    (ratio * dot_rows as f64).round() as usize
}

fn mask_for_col(dots_here: usize, is_right: bool) -> u8 {
    match (dots_here, is_right) {
        (0, _) => 0x00,
        (1, false) => 0x40,
        (2, false) => 0x44,
        (3, false) => 0x46,
        (4, false) => 0x47,
        (1, true) => 0x80,
        (2, true) => 0xA0,
        (3, true) => 0xB0,
        (4, true) => 0xB8,
        _ => 0x00, // dots is clamped to 0..=4, so this is unreachable in practice
    }
}

fn dots_in_row_combined(
    filled_left: usize,
    filled_right: usize,
    row_index_from_bottom: usize,
) -> u8 {
    let row_base = row_index_from_bottom * 4;
    let dots_left = filled_left.saturating_sub(row_base).min(4);
    let dots_right = filled_right.saturating_sub(row_base).min(4);

    mask_for_col(dots_left, false) | mask_for_col(dots_right, true)
}

fn braille_char(mask: u8) -> char {
    char::from_u32(0x2800 + mask as u32).unwrap_or(' ')
}

impl<'a> Widget for BrailleGraph<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 {
            return;
        }

        let dot_rows = area.height as usize * 4;
        let available_cols = area.width as usize;

        // Clear the area first to prevent ghosting from old data
        for y in area.top()..area.bottom() {
            for x in area.left()..area.right() {
                if let Some(cell) = buf.cell_mut((x, y)) {
                    cell.set_char(' ');
                }
            }
        }

        for col in 0..available_cols {
            let left_idx = col * 2;
            let right_idx = col * 2 + 1;

            let s_left = self.data.get(left_idx).copied().unwrap_or(self.min);
            let s_right = self.data.get(right_idx).copied().unwrap_or(self.min);

            let filled_left = normalize(s_left, self.min, self.max, dot_rows);
            let filled_right = normalize(s_right, self.min, self.max, dot_rows);

            for row in 0..(area.height as usize) {
                let mask = dots_in_row_combined(filled_left, filled_right, row);
                if mask == 0 {
                    continue; // Already cleared to ' ' above
                }

                let ch = braille_char(mask);
                let color = gradient_color(
                    s_right,
                    self.min,
                    self.max,
                    self.color_safe,
                    self.color_warn,
                    self.color_crit,
                );

                let y = area.y + (area.height as usize - 1 - row) as u16;
                let x = area.x + col as u16;

                if let Some(cell) = buf.cell_mut((x, y)) {
                    cell.set_char(ch).set_fg(color);
                }
            }
        }
    }
}
