//! Drifting topographic map: contour lines of a slowly morphing height
//! field, drawn in braille (2×4 dots per cell).
//!
//! The field is a sum of plane waves. `sin(k·p + φ)` splits into
//! `sin(kx·x)·cos(ky·y + φ) + cos(kx·x)·sin(ky·y + φ)`, so each wave needs
//! only one row and one column of trig per frame and the per-dot work is a
//! few multiply-adds: a full 200×50 terminal (80k dots) costs well under a
//! millisecond.
//!
//! System load speeds the drift up; music adds a ripple spreading from the
//! centre of the map.

use std::f32::consts::TAU;
use std::sync::Mutex;
use std::time::Instant;

use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::Frame;

use crate::theme::{blend, Theme};

const BRAILLE_LEFT: [u8; 4] = [0x01, 0x02, 0x04, 0x40];
const BRAILLE_RIGHT: [u8; 4] = [0x08, 0x10, 0x20, 0x80];

/// (wavelength in dots, direction in radians, amplitude, phase speed).
/// Incommensurate wavelengths and speeds keep the pattern from repeating.
const WAVES: [(f32, f32, f32, f32); 6] = [
    (170.0, 0.35, 1.00, 0.11),
    (113.0, 1.95, 0.75, -0.08),
    (71.0, 3.05, 0.50, 0.14),
    (47.0, 4.40, 0.32, -0.19),
    (29.0, 5.60, 0.20, 0.26),
    (19.0, 2.60, 0.10, -0.33),
];
/// Height between contour lines (field is normalised to about -1..1).
const STEP: f32 = 0.16;
/// Every Nth contour is an "index contour", drawn brighter like on a map.
const INDEX_EVERY: i32 = 4;

struct State {
    /// Integrated drift. Integrating `speed · dt` (rather than computing
    /// `speed · t`) keeps the map from jumping when the load changes.
    phase: f32,
    last: Option<Instant>,
    /// Smoothed music loudness, 0..1.
    energy: f32,
    /// Ripple clock; only advances while there's music.
    ripple: f32,
    field: Vec<f32>,
    levels: Vec<i32>,
}

static STATE: Mutex<State> = Mutex::new(State {
    phase: 0.0,
    last: None,
    energy: 0.0,
    ripple: 0.0,
    field: Vec::new(),
    levels: Vec::new(),
});

/// Drift speed in phase units per second: calm when idle, livelier under load.
fn speed(cpu_pct: f32) -> f32 {
    (0.35 + (cpu_pct / 100.0).clamp(0.0, 1.0) * 1.6) * 0.2
}

/// Frames per second needed to show the drift smoothly: the fastest a
/// contour can travel is set by the longest-reaching wave, and redrawing
/// faster than ~1.5 frames per dot of travel changes nothing on screen.
fn drift_fps(speed: f32) -> u32 {
    let dots_per_sec = WAVES
        .iter()
        .map(|&(l, _, _, s)| l * s.abs())
        .fold(0.0, f32::max)
        * speed;
    ((dots_per_sec * 3.0).ceil() as u32).clamp(15, 30)
}

/// Fill `field` (w × h, row-major) with the height map at `phase`, plus a
/// ripple of strength `energy` centred on the map.
fn fill(field: &mut Vec<f32>, w: usize, h: usize, phase: f32, energy: f32, ripple: f32) {
    field.clear();
    field.resize(w * h, 0.0);
    // Keep hills a similar size on screen whatever the terminal size.
    let scale = (w.max(h) as f32 / 400.0).clamp(0.85, 1.6);
    let norm: f32 = WAVES.iter().map(|w| w.2).sum();
    let (mut sx, mut cx) = (vec![0.0f32; w], vec![0.0f32; w]);
    for &(lambda, dir, amp, spd) in &WAVES {
        let k = TAU / (lambda * scale);
        let (kx, ky) = (k * dir.cos(), k * dir.sin());
        let a = amp / norm;
        for x in 0..w {
            let (s, c) = (kx * x as f32).sin_cos();
            sx[x] = s * a;
            cx[x] = c * a;
        }
        let phi = phase * spd * TAU;
        for y in 0..h {
            let (sy, cy) = (ky * y as f32 + phi).sin_cos();
            let row = &mut field[y * w..(y + 1) * w];
            for x in 0..w {
                row[x] += sx[x] * cy + cx[x] * sy;
            }
        }
    }
    if energy > 0.01 {
        let (mx, my) = (w as f32 / 2.0, h as f32 / 2.0);
        let reach = w.max(h) as f32 * 0.45;
        for y in 0..h {
            for x in 0..w {
                let r = ((x as f32 - mx).powi(2) + (y as f32 - my).powi(2)).sqrt();
                field[y * w + x] +=
                    energy * 0.8 * (r / (6.0 * scale) - ripple).sin() * (-r / reach).exp();
            }
        }
    }
}

/// Braille bits and "is an index contour" for the cell at (cx, cy).
fn cell_bits(levels: &[i32], w: usize, cx: usize, cy: usize) -> (u8, bool) {
    let (mut bits, mut index) = (0u8, false);
    for dy in 0..4 {
        for dx in 0..2 {
            let (x, y) = (cx * 2 + dx, cy * 4 + dy);
            let q = levels[y * w + x];
            let (qr, qd) = (levels[y * w + x + 1], levels[(y + 1) * w + x]);
            if q != qr || q != qd {
                bits |= if dx == 0 {
                    BRAILLE_LEFT[dy]
                } else {
                    BRAILLE_RIGHT[dy]
                };
                let crossed = q.max(if q != qr { qr } else { qd });
                index |= crossed.rem_euclid(INDEX_EVERY) == 0;
            }
        }
    }
    (bits, index)
}

/// Draw the map over `area`. `motion` false freezes it (the `o` key); at
/// night it drifts at a lower frame rate.
pub fn render(f: &mut Frame, area: Rect, theme: &Theme, cpu_pct: f32, motion: bool, night: bool) {
    if area.width < 4 || area.height < 2 {
        return;
    }
    crate::widgets::music_viz::ensure_running();
    let mut st = STATE.lock().unwrap_or_else(|e| e.into_inner());
    let now = Instant::now();
    // Clamp so returning to the scene after a while doesn't lurch forward.
    let dt = st
        .last
        .replace(now)
        .map_or(0.0, |t| now.duration_since(t).as_secs_f32().min(0.5));
    let target = crate::widgets::music_viz::energy();
    let tau = if target > st.energy { 0.15 } else { 1.2 };
    st.energy += (target - st.energy) * (1.0 - (-dt / tau).exp());
    if motion {
        let v = speed(cpu_pct);
        st.phase += v * dt;
        st.ripple += dt * 3.0 * (st.energy > 0.01) as u8 as f32;
        let fps = if st.energy > 0.02 { 30 } else { drift_fps(v) };
        // Night: the same drift, just fewer (and dimmer) redraws.
        crate::anim::request(if night { fps.min(4) } else { fps });
    }

    let (cols, rows) = (area.width as usize, area.height as usize);
    let (w, h) = (cols * 2 + 1, rows * 4 + 1);
    let (phase, energy, ripple) = (st.phase, st.energy, st.ripple);
    let State { field, levels, .. } = &mut *st;
    fill(field, w, h, phase, energy, ripple);
    levels.clear();
    levels.extend(field.iter().map(|v| (v / STEP).floor() as i32));

    let low = blend(theme.secondary, theme.bg, 0.35);
    let high = theme.accent;
    let buf = f.buffer_mut();
    for cy in 0..rows {
        for cx in 0..cols {
            let (bits, index) = cell_bits(levels, w, cx, cy);
            if bits == 0 {
                continue;
            }
            let height = field[(cy * 4 + 2) * w + cx * 2 + 1];
            let base = blend(low, high, ((height + 1.0) / 2.0).clamp(0.0, 1.0));
            let color: Color = if index {
                blend(base, theme.text, 0.3)
            } else {
                blend(base, theme.bg, 0.4)
            };
            if let Some(cell) = buf.cell_mut((area.x + cx as u16, area.y + cy as u16)) {
                cell.set_char(char::from_u32(0x2800 + bits as u32).unwrap_or(' '))
                    .set_style(Style::default().fg(color));
            }
        }
    }
}

/// Whether music is currently shaping the map (for the scene caption).
pub fn reacting_to_music() -> bool {
    STATE.lock().is_ok_and(|s| s.energy > 0.02)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn separable_waves_match_the_direct_formula() {
        let (w, h) = (40, 20);
        let mut field = Vec::new();
        fill(&mut field, w, h, 3.7, 0.0, 0.0);
        let scale = (w.max(h) as f32 / 400.0).clamp(0.85, 1.6);
        let norm: f32 = WAVES.iter().map(|w| w.2).sum();
        for &(x, y) in &[(0, 0), (13, 7), (39, 19)] {
            let direct: f32 = WAVES
                .iter()
                .map(|&(l, d, a, s)| {
                    let k = TAU / (l * scale);
                    a / norm
                        * (k * d.cos() * x as f32 + k * d.sin() * y as f32 + 3.7 * s * TAU).sin()
                })
                .sum();
            assert!((field[y * w + x] - direct).abs() < 1e-4);
        }
        assert!(field.iter().all(|v| v.abs() <= 1.0 + 1e-4));
    }

    #[test]
    fn music_ripple_is_strongest_near_the_centre() {
        let (w, h) = (240, 120);
        let (mut calm, mut rippled) = (Vec::new(), Vec::new());
        fill(&mut calm, w, h, 1.0, 0.0, 0.0);
        fill(&mut rippled, w, h, 1.0, 1.0, 0.0);
        let mean_diff = |near: bool| {
            let (mut sum, mut n) = (0.0, 0);
            for y in 0..h {
                for x in 0..w {
                    let r = ((x as f32 - 120.0).powi(2) + (y as f32 - 60.0).powi(2)).sqrt();
                    if (r < 30.0) == near && (near || r > 110.0) {
                        sum += (rippled[y * w + x] - calm[y * w + x]).abs();
                        n += 1;
                    }
                }
            }
            sum / n as f32
        };
        assert!(mean_diff(true) > 0.2);
        assert!(mean_diff(true) > mean_diff(false) * 2.0);
    }

    #[test]
    fn contours_appear_where_levels_change() {
        // Two cells wide, one tall; level steps between dot columns 1 and 2.
        let w = 5;
        let levels: Vec<i32> = (0..5 * w).map(|i| if i % w < 2 { 0 } else { 1 }).collect();
        let (bits, index) = cell_bits(&levels, w, 0, 0);
        assert_eq!(bits, 0x08 | 0x10 | 0x20 | 0x80, "right dot column lit");
        assert!(!index);
        let (bits, _) = cell_bits(&levels, w, 1, 0);
        assert_eq!(bits, 0, "flat ground has no lines");
        let levels: Vec<i32> = levels.iter().map(|q| q + 3).collect();
        assert!(
            cell_bits(&levels, w, 0, 0).1,
            "crossing level 4 is an index contour"
        );
    }

    #[test]
    fn load_speeds_the_drift_up() {
        assert!(speed(90.0) > speed(5.0) * 2.0);
        assert_eq!(speed(500.0), speed(100.0));
        // Idle drift is slow enough for a few fps; heavy load asks for more.
        assert!(drift_fps(speed(5.0)) <= 20);
        assert!(drift_fps(speed(100.0)) > drift_fps(speed(5.0)));
    }
}
