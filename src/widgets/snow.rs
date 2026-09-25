use std::sync::{LazyLock, Mutex};
use std::time::Instant;

use ratatui::buffer::Buffer;
use ratatui::layout::{Position, Rect};
use ratatui::style::Style;
use ratatui::widgets::Widget;
use ratatui::Frame;

use crate::theme::Theme;

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
    fn bool(&mut self, threshold: f32) -> bool {
        ((self.next() >> 40) as f32 / (1u64 << 24) as f32) < threshold
    }
    fn range(&mut self, min: f32, max: f32) -> f32 {
        min + ((self.next() >> 40) as f32 / (1u64 << 24) as f32) * (max - min)
    }
}

struct Flake {
    x: f32,
    y: f32,
    speed: f32,
    drift: f32,
}

struct SnowState {
    flakes: Vec<Flake>,
    piled: Vec<u16>,
    last_tick: Instant,
    rng: Rng,
    width: u16,
    height: u16,
}

impl SnowState {
    fn new() -> Self {
        Self {
            flakes: Vec::new(),
            piled: Vec::new(),
            last_tick: Instant::now(),
            rng: Rng(0x847293847),
            width: 0,
            height: 0,
        }
    }

    fn resize(&mut self, w: u16, h: u16) {
        if self.width != w || self.height != h || self.piled.is_empty() {
            self.width = w;
            self.height = h;
            self.piled.resize(w as usize, 0);

            self.flakes.clear();
            for _ in 0..(w * h / 20) {
                self.flakes.push(Flake {
                    x: self.rng.range(0.0, w as f32),
                    y: self.rng.range(0.0, h as f32),
                    speed: self.rng.range(5.0, 15.0),
                    drift: self.rng.range(-2.0, 2.0),
                });
            }
        }
    }
}

static STATE: LazyLock<Mutex<SnowState>> = LazyLock::new(|| Mutex::new(SnowState::new()));

pub fn render(f: &mut Frame, area: Rect, theme: &Theme) {
    if area.width < 5 || area.height < 5 {
        return;
    }

    let mut st = STATE.lock().unwrap();
    st.resize(area.width, area.height);

    let now = Instant::now();
    let dt = now.duration_since(st.last_tick).as_secs_f32().min(0.1);
    st.last_tick = now;

    let h = st.height;
    let w = st.width;

    for i in 0..st.flakes.len() {
        let flake = &st.flakes[i];
        let mut x = flake.x + flake.drift * dt;
        let mut y = flake.y + flake.speed * dt;

        if x < 0.0 {
            x += w as f32;
        } else if x >= w as f32 {
            x -= w as f32;
        }

        let ix = (x as usize).min((w - 1) as usize);
        let pile = st.piled[ix];
        let max_y = h.saturating_sub(pile).saturating_sub(1) as f32;

        if y >= max_y {
            if pile < h / 2 {
                st.piled[ix] += 1;
            }
            y = -1.0;
            x = st.rng.range(0.0, w as f32);
        }

        st.flakes[i].x = x;
        st.flakes[i].y = y;
    }

    if st.rng.bool(0.1) {
        let ix = (st.rng.next() % w as u64) as usize;
        if st.piled[ix] > 0 {
            st.piled[ix] -= 1;
        }
    }

    crate::anim::request_full();

    let widget = SnowWidget {
        state: &mut st,
        theme,
    };
    f.render_widget(widget, area);
}

struct SnowWidget<'a> {
    state: &'a mut SnowState,
    theme: &'a Theme,
}

impl<'a> Widget for SnowWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let w = self.state.width;
        let h = self.state.height;

        for x in 0..w {
            let pile = self.state.piled[x as usize];
            for p in 0..pile {
                let y = h.saturating_sub(p).saturating_sub(1);
                if let Some(c) = buf.cell_mut(Position {
                    x: area.x + x,
                    y: area.y + y,
                }) {
                    c.set_char('▇')
                        .set_style(Style::default().fg(self.theme.secondary));
                }
            }
        }

        for flake in &self.state.flakes {
            let cx = (flake.x as u16).min(w.saturating_sub(1));
            let cy = (flake.y as u16).min(h.saturating_sub(1));

            if let Some(c) = buf.cell_mut(Position {
                x: area.x + cx,
                y: area.y + cy,
            }) {
                c.set_char(if flake.speed > 10.0 { '*' } else { '.' })
                    .set_style(Style::default().fg(self.theme.text));
            }
        }
    }
}
