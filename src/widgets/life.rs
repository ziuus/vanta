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
}

struct LifeState {
    grid: Vec<bool>,
    next_grid: Vec<bool>,
    width: u16,
    height: u16,
    last_tick: Instant,
    rng: Rng,
}

impl LifeState {
    fn new() -> Self {
        Self {
            grid: Vec::new(),
            next_grid: Vec::new(),
            width: 0,
            height: 0,
            last_tick: Instant::now(),
            rng: Rng(0x987654321),
        }
    }

    fn resize_and_seed(&mut self, w: u16, h: u16) {
        if self.width != w || self.height != h || self.grid.is_empty() {
            self.width = w;
            self.height = h;
            self.grid.resize((w as usize) * (h as usize), false);
            self.next_grid.resize((w as usize) * (h as usize), false);

            for i in 0..self.grid.len() {
                self.grid[i] = self.rng.bool(0.15);
            }
        }
    }

    fn step(&mut self) -> bool {
        let mut changed = false;
        let w = self.width as i32;
        let h = self.height as i32;

        for y in 0..h {
            for x in 0..w {
                let mut alive_neighbors = 0;
                for dy in -1..=1 {
                    for dx in -1..=1 {
                        if dx == 0 && dy == 0 {
                            continue;
                        }
                        let nx = (x + dx).rem_euclid(w);
                        let ny = (y + dy).rem_euclid(h);
                        if self.grid[(ny * w + nx) as usize] {
                            alive_neighbors += 1;
                        }
                    }
                }

                let idx = (y * w + x) as usize;
                let is_alive = self.grid[idx];
                let next_alive = match (is_alive, alive_neighbors) {
                    (true, 2) | (true, 3) => true,
                    (false, 3) => true,
                    _ => false,
                };

                if next_alive != is_alive {
                    changed = true;
                }
                self.next_grid[idx] = next_alive;
            }
        }

        std::mem::swap(&mut self.grid, &mut self.next_grid);

        if !changed {
            for _ in 0..10 {
                let idx = (self.rng.next() % self.grid.len().max(1) as u64) as usize;
                if idx < self.grid.len() {
                    self.grid[idx] = true;
                }
            }
        }

        changed
    }
}

static STATE: LazyLock<Mutex<LifeState>> = LazyLock::new(|| Mutex::new(LifeState::new()));

pub fn render(f: &mut Frame, area: Rect, theme: &Theme) {
    if area.width < 5 || area.height < 5 {
        return;
    }

    let mut st = STATE.lock().unwrap();
    st.resize_and_seed(area.width, area.height);

    let now = Instant::now();
    if now.duration_since(st.last_tick).as_secs_f32() > 0.1 {
        st.last_tick = now;
        st.step();
    }

    crate::anim::request_full();

    let widget = LifeWidget {
        state: &mut st,
        theme,
    };
    f.render_widget(widget, area);
}

struct LifeWidget<'a> {
    state: &'a mut LifeState,
    theme: &'a Theme,
}

impl<'a> Widget for LifeWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let w = self.state.width;
        let h = self.state.height;
        for y in 0..h {
            for x in 0..w {
                if self.state.grid[(y * w + x) as usize] {
                    if let Some(c) = buf.cell_mut(Position {
                        x: area.x + x,
                        y: area.y + y,
                    }) {
                        c.set_char('■')
                            .set_style(Style::default().fg(self.theme.secondary));
                    }
                }
            }
        }
    }
}
