use std::sync::{LazyLock, Mutex};
use std::time::Instant;

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
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
    fn unit(&mut self) -> f32 {
        (self.next() >> 40) as f32 / (1u64 << 24) as f32
    }
    fn range(&mut self, min: f32, max: f32) -> f32 {
        min + self.unit() * (max - min)
    }
}

struct Star {
    x: f32,
    y: f32,
    z: f32,
    prev_z: f32,
}

struct StarfieldState {
    stars: Vec<Star>,
    last_tick: Instant,
    rng: Rng,
    current_speed: f32,
}

impl StarfieldState {
    fn new(count: usize) -> Self {
        let mut rng = Rng(0x123456789ABCDEF);
        let mut stars = Vec::with_capacity(count);
        for _ in 0..count {
            stars.push(Star {
                x: rng.range(-1.0, 1.0),
                y: rng.range(-1.0, 1.0),
                z: rng.range(0.1, 1.0),
                prev_z: 1.0,
            });
        }
        Self {
            stars,
            last_tick: Instant::now(),
            rng,
            current_speed: 0.1,
        }
    }
}

static STATE: LazyLock<Mutex<StarfieldState>> =
    LazyLock::new(|| Mutex::new(StarfieldState::new(300)));

pub fn render(f: &mut Frame, area: Rect, theme: &Theme) {
    if area.width < 5 || area.height < 5 {
        return;
    }

    let mut st = STATE.lock().unwrap();
    let now = Instant::now();
    let dt = now.duration_since(st.last_tick).as_secs_f32().min(0.1);
    st.last_tick = now;

    let cpu = crate::monitors::cpu::snapshot().usage as f32 / 100.0;
    let music = crate::widgets::music_viz::energy();
    let target_speed = 0.05 + (cpu * 0.5) + (music * 1.5);

    st.current_speed += (target_speed - st.current_speed) * (dt * 5.0);

    let speed = st.current_speed;

    if speed > 0.1 {
        crate::anim::request_full();
    }

    let widget = StarfieldWidget {
        state: &mut st,
        dt,
        theme,
        speed,
    };
    f.render_widget(widget, area);
}

struct StarfieldWidget<'a> {
    state: &'a mut StarfieldState,
    dt: f32,
    theme: &'a Theme,
    speed: f32,
}

impl<'a> Widget for StarfieldWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let cx = area.x as f32 + area.width as f32 / 2.0;
        let cy = area.y as f32 + area.height as f32 / 2.0;
        let aspect = 2.0;

        let mut i = 0;
        while i < self.state.stars.len() {
            let star = &mut self.state.stars[i];
            star.prev_z = star.z;
            star.z -= self.speed * self.dt;

            if star.z <= 0.01 {
                star.x = self.state.rng.range(-1.0, 1.0);
                star.y = self.state.rng.range(-1.0, 1.0);
                star.z = 1.0;
                star.prev_z = 1.0;
            }

            let inv_z = 1.0 / star.z;
            let sx = cx + (star.x * inv_z * area.width as f32 * 0.5);
            let sy = cy + (star.y * inv_z * area.height as f32 * 0.5 / aspect);

            if sx >= area.x as f32
                && sx < (area.x + area.width) as f32
                && sy >= area.y as f32
                && sy < (area.y + area.height) as f32
            {
                let cell_x = sx as u16;
                let cell_y = sy as u16;

                let intensity = (1.0 - star.z).clamp(0.0, 1.0);

                let color = if intensity > 0.8 {
                    self.theme.text
                } else if intensity > 0.4 {
                    self.theme.secondary
                } else {
                    self.theme.dim
                };

                let char_idx = if self.speed > 0.5 {
                    let dx = sx - cx;
                    let dy = sy - cy;
                    if dx.abs() > dy.abs() {
                        '-'
                    } else {
                        '|'
                    }
                } else {
                    if intensity > 0.8 {
                        '*'
                    } else if intensity > 0.5 {
                        '+'
                    } else {
                        '.'
                    }
                };

                buf.cell_mut(ratatui::layout::Position {
                    x: cell_x,
                    y: cell_y,
                })
                .unwrap()
                .set_char(char_idx)
                .set_style(Style::default().fg(color));
            }

            i += 1;
        }
    }
}
