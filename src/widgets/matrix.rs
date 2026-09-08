use std::sync::{LazyLock, Mutex};
use std::time::Instant;

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::Widget;
use ratatui::Frame;

use crate::theme::Theme;

const CHARSET: &[char] = &[
    'ｱ', 'ｲ', 'ｳ', 'ｴ', 'ｵ', 'ｶ', 'ｷ', 'ｸ', 'ｹ', 'ｺ', 'ｻ', 'ｼ', 'ｽ', 'ｾ', 'ｿ', 'ﾀ', 'ﾁ', 'ﾂ', 'ﾃ',
    'ﾄ', 'ﾅ', 'ﾆ', 'ﾇ', 'ﾈ', 'ﾉ', 'ﾊ', 'ﾋ', 'ﾌ', 'ﾍ', 'ﾎ', 'ﾏ', 'ﾐ', 'ﾑ', 'ﾒ', 'ﾓ', 'ﾔ', 'ﾕ', 'ﾖ',
    'ﾗ', 'ﾘ', 'ﾙ', 'ﾚ', 'ﾛ', 'ﾜ', 'ｦ', 'ﾝ', '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'Z',
    'X', 'V', 'K', '<', '>', '*', '+', '=', ':', '¦',
];

/// xorshift64* — tiny, fast, no dependency.
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n.max(1) as u64) as usize
    }
    fn unit(&mut self) -> f32 {
        (self.next() >> 40) as f32 / (1u64 << 24) as f32
    }
}

struct Drop {
    /// Row of the bright head, in cells (fractional for smooth speed).
    head: f32,
    speed: f32,
    len: usize,
    /// Per-cell glyphs; the head mutates its glyph as it falls.
    glyphs: Vec<char>,
    /// Frames until this column respawns after the tail leaves the screen.
    wait: u16,
}

struct State {
    rng: Rng,
    cols: Vec<Drop>,
    width: u16,
    height: u16,
    last: Instant,
}

static STATE: LazyLock<Mutex<State>> = LazyLock::new(|| {
    Mutex::new(State {
        rng: Rng(0x9E37_79B9_7F4A_7C15),
        cols: Vec::new(),
        width: 0,
        height: 0,
        last: Instant::now(),
    })
});

fn new_drop(rng: &mut Rng, height: u16, initial: bool) -> Drop {
    let len = 4 + rng.below((height as usize / 2).max(4));
    let glyphs = (0..len + 2)
        .map(|_| CHARSET[rng.below(CHARSET.len())])
        .collect();
    Drop {
        // Stagger initial heads so the first frame isn't a flat line.
        head: if initial {
            -(rng.below(height as usize * 2) as f32)
        } else {
            -(len as f32)
        },
        speed: 6.0 + rng.unit() * 14.0, // rows per second
        len,
        glyphs,
        wait: rng.below(40) as u16,
    }
}

fn advance(st: &mut State, area: Rect) {
    let now = Instant::now();
    let dt = now.duration_since(st.last).as_secs_f32().min(0.25);
    st.last = now;

    if st.width != area.width || st.height != area.height {
        st.width = area.width;
        st.height = area.height;
        st.cols = (0..area.width)
            .map(|_| new_drop(&mut st.rng, area.height, true))
            .collect();
    }
    let h = area.height as f32;
    for i in 0..st.cols.len() {
        let d = &mut st.cols[i];
        if d.wait > 0 {
            d.wait -= 1;
            continue;
        }
        let before = d.head as i32;
        d.head += d.speed * dt;
        if d.head as i32 != before {
            // Head moved a row: roll a fresh glyph into the head slot.
            let n = d.glyphs.len();
            d.glyphs.rotate_right(1);
            d.glyphs[0] = CHARSET[st.rng.below(CHARSET.len())];
            // Occasionally flicker one glyph in the tail.
            if st.rng.below(3) == 0 {
                let k = st.rng.below(n);
                d.glyphs[k] = CHARSET[st.rng.below(CHARSET.len())];
            }
        }
        if d.head - d.len as f32 > h {
            let h_cells = st.height;
            st.cols[i] = new_drop(&mut st.rng, h_cells, false);
        }
    }
}

struct Rain<'a> {
    theme: &'a Theme,
}

impl Widget for Rain<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let mut st = STATE.lock().unwrap();
        advance(&mut st, area);
        let (ar, ag, ab) = match self.theme.accent {
            Color::Rgb(r, g, b) => (r as f32, g as f32, b as f32),
            _ => (120.0, 220.0, 150.0),
        };
        let (br, bg, bb) = match self.theme.bg {
            Color::Rgb(r, g, b) => (r as f32, g as f32, b as f32),
            _ => (10.0, 10.0, 15.0),
        };
        for (x, d) in st.cols.iter().enumerate() {
            if d.wait > 0 {
                continue;
            }
            let head = d.head.floor() as i32;
            for k in 0..d.len {
                let y = head - k as i32;
                if y < 0 || y >= area.height as i32 {
                    continue;
                }
                let ch = d.glyphs[k.min(d.glyphs.len() - 1)];
                // Fade from bright head to background along the tail.
                let t = 1.0 - k as f32 / d.len as f32;
                let t = t * t;
                let style = if k == 0 {
                    Style::default()
                        .fg(self.theme.text)
                        .add_modifier(Modifier::BOLD)
                } else {
                    let mix = |a: f32, b: f32| (b + (a - b) * t) as u8;
                    Style::default().fg(Color::Rgb(mix(ar, br), mix(ag, bg), mix(ab, bb)))
                };
                if let Some(cell) = buf.cell_mut((area.x + x as u16, area.y + y as u16)) {
                    cell.set_char(ch);
                    cell.set_style(style);
                }
            }
        }
    }
}

pub fn render(f: &mut Frame, area: Rect, theme: &Theme) {
    if area.width < 4 || area.height < 3 {
        return;
    }
    f.render_widget(Rain { theme }, area);
}
