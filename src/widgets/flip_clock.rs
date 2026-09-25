//! Retro flip clock: two cards (hours, minutes) whose top leaf folds down
//! over the hinge when the minute changes.
//!
//! Digits are drawn with half-blocks, so one "pixel" is a column by half a
//! row: square on a typical terminal font, and twice the vertical resolution
//! of the block-glyph clock. The animation is derived from wall time (the
//! first [`FLIP_MS`] of every minute), so there is no state to get stuck.

use chrono::{DateTime, Local, Timelike};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::Frame;

use crate::theme::{blend, Theme};

const FONT_W: usize = 7;
const FONT_H: usize = 12;
const FLIP_MS: u32 = 700;

/// Bold digits with 2-pixel strokes and rounded corners. Every stroke is
/// exactly two pixels, so any half-pixel scale keeps strokes a whole number
/// of cells wide.
#[rustfmt::skip]
const DIGITS: [[&str; FONT_H]; 10] = [
    [".#####.", "#######", "##...##", "##...##", "##...##", "##...##", "##...##", "##...##", "##...##", "##...##", "#######", ".#####."],
    ["...##..", "..###..", ".####..", "...##..", "...##..", "...##..", "...##..", "...##..", "...##..", "...##..", ".######", ".######"],
    [".#####.", "#######", "##...##", ".....##", "....###", "...###.", "..###..", ".###...", "###....", "##.....", "#######", "#######"],
    [".#####.", "#######", "##...##", ".....##", ".....##", "..####.", "..####.", ".....##", ".....##", "##...##", "#######", ".#####."],
    ["....###", "...####", "..##.##", ".##..##", "##...##", "##...##", "#######", "#######", ".....##", ".....##", ".....##", ".....##"],
    ["#######", "#######", "##.....", "##.....", "######.", "#######", ".....##", ".....##", ".....##", "##...##", "#######", ".#####."],
    [".#####.", "#######", "##...##", "##.....", "######.", "#######", "##...##", "##...##", "##...##", "##...##", "#######", ".#####."],
    ["#######", "#######", ".....##", ".....##", "....##.", "....##.", "...##..", "...##..", "..##...", "..##...", "..##...", "..##..."],
    [".#####.", "#######", "##...##", "##...##", "##...##", ".#####.", ".#####.", "##...##", "##...##", "##...##", "#######", ".#####."],
    [".#####.", "#######", "##...##", "##...##", "##...##", "#######", ".######", ".....##", ".....##", "##...##", "#######", ".#####."],
];

fn font_lit(c: char, px: usize, py: usize) -> bool {
    let Some(d) = c.to_digit(10) else {
        return false;
    };
    px < FONT_W && py < FONT_H && DIGITS[d as usize][py].as_bytes()[px] == b'#'
}

/// Size of one card. `s` is the scale in half-pixels: one font pixel is
/// `s / 2` columns wide and `s / 2` half-rows tall.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Geom {
    s: usize,
    /// One digit's width in columns and height in half-rows.
    dw: usize,
    dh: usize,
    /// Card width in columns.
    w: usize,
    /// Height of each leaf (above and below the hinge) in half-rows. Even,
    /// so a leaf is a whole number of cells.
    leaf: usize,
}

impl Geom {
    fn new(s: usize) -> Self {
        let dw = (FONT_W * s).div_ceil(2);
        let dh = (FONT_H * s).div_ceil(2);
        Self {
            s,
            dw,
            dh,
            w: 2 * dw + s + 2 * s,
            leaf: (dh + 3 * s / 2).div_ceil(4) * 2,
        }
    }
    /// Card height in rows: two leaves plus the hinge row.
    fn h(&self) -> usize {
        self.leaf + 1
    }
    fn gap(&self) -> usize {
        self.s
    }
    fn total_w(&self) -> usize {
        2 * self.w + self.gap()
    }
    /// Is pixel (x, y) of `text` lit? `x` in card columns, `y` in half-rows
    /// counted from the top of the upper leaf. Text is centred on the card,
    /// so a one-digit hour sits in the middle.
    fn lit(&self, text: &str, x: usize, y: usize) -> bool {
        let n = text.chars().count();
        let cw = n * self.dw + n.saturating_sub(1) * self.s;
        let ox = self.w.saturating_sub(cw) / 2;
        let oy = (2 * self.leaf).saturating_sub(self.dh) / 2;
        let (Some(x), Some(y)) = (x.checked_sub(ox), y.checked_sub(oy)) else {
            return false;
        };
        let slot = self.dw + self.s;
        let (i, dx) = (x / slot, x % slot);
        dx < self.dw
            && y < self.dh
            && text
                .chars()
                .nth(i)
                .is_some_and(|c| font_lit(c, dx * 2 / self.s, y * 2 / self.s))
    }
}

/// Largest scale whose two cards (plus a line for the date) fit.
fn pick(w: u16, h: u16) -> Option<Geom> {
    (2..=16)
        .rev()
        .map(Geom::new)
        .find(|g| g.total_w() <= w as usize && g.h() + 2 <= h as usize)
}

fn ease_in(t: f32) -> f32 {
    t * t
}

fn ease_out(t: f32) -> f32 {
    1.0 - (1.0 - t).powi(3)
}

/// Where the flap is in `[0, 1)` during the first [`FLIP_MS`] of a minute.
fn flip_progress(now: &DateTime<Local>) -> Option<f32> {
    let ms = now.second() * 1000 + now.timestamp_subsec_millis().min(999);
    (ms < FLIP_MS).then(|| ms as f32 / FLIP_MS as f32)
}

struct Palette {
    digit: Color,
    top: Color,
    bottom: Color,
    hinge: Color,
    outside: Color,
}

/// Colour of each half-row pixel; `None` is outside the card.
type Px = Option<Color>;

fn put(buf: &mut Buffer, x: u16, y: u16, top: Px, bottom: Px, outside: Color) {
    let Some(cell) = buf.cell_mut((x, y)) else {
        return;
    };
    match (top, bottom) {
        (Some(t), Some(b)) => {
            cell.set_char('▀').set_style(Style::default().fg(t).bg(b));
        }
        (Some(t), None) => {
            cell.set_char('▀')
                .set_style(Style::default().fg(t).bg(outside));
        }
        (None, Some(b)) => {
            cell.set_char('▄')
                .set_style(Style::default().fg(b).bg(outside));
        }
        (None, None) => {}
    }
}

/// Draw one card. `old`/`new` are the card's text before and after the
/// minute boundary (equal when nothing is flipping); `p` is flip progress.
fn card(
    buf: &mut Buffer,
    (x0, y0): (u16, u16),
    g: Geom,
    (old, new): (&str, &str),
    p: Option<f32>,
    pal: &Palette,
) {
    let leaf = g.leaf as f32;
    let p = p.filter(|_| old != new);
    // Flap shading: the leaf darkens as it tilts away from the viewer.
    let shade = |c: Color, s: f32| blend(c, pal.hinge, (1.0 - s) * 0.55);
    for col in 0..g.w {
        let corner = col == 0 || col + 1 == g.w;
        // Upper leaf: half-rows 0..leaf.
        let upper = |hr: usize| -> Px {
            if corner && hr == 0 {
                return None;
            }
            let static_px = |text: &str| {
                Some(if g.lit(text, col, hr) {
                    pal.digit
                } else {
                    pal.top
                })
            };
            match p {
                Some(p) if p < 0.5 => {
                    let s = 1.0 - ease_in(p / 0.5);
                    let d = leaf - (hr as f32 + 0.5);
                    if d < leaf * s {
                        let src = (leaf - d / s).max(0.0) as usize;
                        let c = if g.lit(old, col, src) {
                            pal.digit
                        } else {
                            pal.top
                        };
                        Some(shade(c, s))
                    } else {
                        static_px(new)
                    }
                }
                _ => static_px(new),
            }
        };
        // Lower leaf: half-rows leaf..2*leaf, `hr` counted from the hinge.
        let lower = |hr: usize| -> Px {
            if corner && hr + 1 == g.leaf {
                return None;
            }
            let y = g.leaf + hr;
            let static_px = |text: &str| {
                Some(if g.lit(text, col, y) {
                    pal.digit
                } else {
                    pal.bottom
                })
            };
            match p {
                Some(p) if p >= 0.5 => {
                    let s = ease_out((p - 0.5) / 0.5).max(0.001);
                    let d = hr as f32 + 0.5;
                    if d < leaf * s {
                        let src = g.leaf + ((d / s) as usize).min(g.leaf - 1);
                        let c = if g.lit(new, col, src) {
                            pal.digit
                        } else {
                            pal.bottom
                        };
                        Some(shade(c, s))
                    } else {
                        static_px(old)
                    }
                }
                Some(_) => static_px(old),
                None => static_px(new),
            }
        };
        let x = x0 + col as u16;
        for r in 0..g.leaf / 2 {
            put(
                buf,
                x,
                y0 + r as u16,
                upper(2 * r),
                upper(2 * r + 1),
                pal.outside,
            );
        }
        // The hinge: a dark seam through the middle, notched at the edges.
        if let Some(cell) = buf.cell_mut((x, y0 + (g.leaf / 2) as u16)) {
            if corner {
                cell.set_char(' ')
                    .set_style(Style::default().bg(pal.outside));
            } else {
                cell.set_char('━')
                    .set_style(Style::default().fg(pal.hinge).bg(pal.bottom));
            }
        }
        let ly = y0 + (g.leaf / 2) as u16 + 1;
        for r in 0..g.leaf / 2 {
            put(
                buf,
                x,
                ly + r as u16,
                lower(2 * r),
                lower(2 * r + 1),
                pal.outside,
            );
        }
    }
}

fn card_texts(t: &DateTime<Local>, h24: bool) -> (String, String) {
    let h = if h24 {
        t.format("%H").to_string()
    } else {
        t.format("%-I").to_string()
    };
    (h, t.format("%M").to_string())
}

/// Draw the clock centred in `area`, with the date (and `note`, if it fits)
/// underneath. `transparent` leaves the page background to the terminal.
pub fn render(
    f: &mut Frame,
    area: Rect,
    theme: &Theme,
    h24: bool,
    transparent: bool,
    note: Option<&str>,
) {
    let now = Local::now();
    let Some(g) = pick(area.width, area.height) else {
        crate::widgets::clock::render(f, area, theme, h24, "standard", "solid", &[]);
        return;
    };

    let p = flip_progress(&now);
    // Wake the loop just before the boundary so the flip starts on time
    // (idle redraws are only every 500ms), then run smoothly through it.
    let to_next = 60_000 - (now.second() * 1000 + now.timestamp_subsec_millis().min(999));
    if p.is_some() || to_next < 600 {
        crate::anim::request(30);
    }

    let prev = now - chrono::Duration::minutes(1);
    let (h_new, m_new) = card_texts(&now, h24);
    let (h_old, m_old) = card_texts(&prev, h24);

    let pal = Palette {
        digit: theme.text,
        // Cards are lifted off the page, the upper leaf catching a little
        // more light than the lower one.
        top: blend(theme.bg, theme.text, 0.10),
        bottom: blend(theme.bg, theme.text, 0.065),
        hinge: theme.bg,
        outside: if transparent { Color::Reset } else { theme.bg },
    };

    let note = note.filter(|_| g.h() + 3 <= area.height as usize);
    let block_h = g.h() + 2 + usize::from(note.is_some());
    let x0 = area.x + (area.width - g.total_w() as u16) / 2;
    let y0 = area.y + (area.height.saturating_sub(block_h as u16)) / 2;
    let buf = f.buffer_mut();
    card(buf, (x0, y0), g, (&h_old, &h_new), p, &pal);
    let mx = x0 + (g.w + g.gap()) as u16;
    card(buf, (mx, y0), g, (&m_old, &m_new), p, &pal);

    // am/pm tucked into the hour card's top-left corner, like the classics.
    if !h24 && g.s >= 4 {
        let tag = now.format("%p").to_string();
        buf.set_string(
            x0 + (g.s / 2).max(2) as u16,
            y0 + 1,
            tag,
            Style::default()
                .fg(blend(pal.top, theme.text, 0.45))
                .bg(pal.top),
        );
    }

    let date = now.format("%A, %-d %B").to_string();
    let dy = y0 + g.h() as u16 + 1;
    let line = |buf: &mut Buffer, y: u16, s: &str, c: Color| {
        let s = crate::widgets::meter::ellipsize(s, area.width as usize);
        let w = s.chars().count() as u16;
        buf.set_string(area.x + (area.width - w) / 2, y, s, Style::default().fg(c));
    };
    line(buf, dy, &date, theme.dim);
    if let Some(n) = note {
        line(buf, dy + 1, n, theme.secondary);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn font_is_rectangular() {
        for d in DIGITS {
            assert!(d.iter().all(|row| row.len() == FONT_W));
        }
    }

    #[test]
    fn largest_scale_that_fits_is_chosen() {
        let g = pick(194, 49).unwrap();
        assert!(g.dh >= 40, "200x50 should be massive, got {g:?}");
        let g = pick(114, 33).unwrap();
        assert!(g.total_w() <= 114 && g.h() + 2 <= 33);
        assert!(pick(10, 4).is_none());
        for s in 2..=16 {
            assert_eq!(Geom::new(s).leaf % 2, 0);
        }
    }

    #[test]
    fn digits_are_centred_and_split_by_the_hinge() {
        let g = Geom::new(4);
        // A single-digit hour is centred: its left edge is further in than
        // the first digit of a two-digit hour.
        let first_lit = |t: &str| (0..g.w).find(|&x| (0..2 * g.leaf).any(|y| g.lit(t, x, y)));
        assert!(first_lit("8").unwrap() > first_lit("88").unwrap());
        // '8' has ink in both leaves.
        assert!((0..g.w).any(|x| (0..g.leaf).any(|y| g.lit("8", x, y))));
        assert!((0..g.w).any(|x| (g.leaf..2 * g.leaf).any(|y| g.lit("8", x, y))));
    }

    #[test]
    fn flip_runs_only_at_the_start_of_a_minute() {
        let at = |s, ms| {
            Local
                .with_ymd_and_hms(2026, 9, 25, 10, 42, s)
                .unwrap()
                .checked_add_signed(chrono::Duration::milliseconds(ms))
                .unwrap()
        };
        assert_eq!(flip_progress(&at(0, 0)), Some(0.0));
        assert!(flip_progress(&at(0, 350)).is_some_and(|p| (p - 0.5).abs() < 0.01));
        assert_eq!(flip_progress(&at(0, 700)), None);
        assert_eq!(flip_progress(&at(30, 0)), None);
    }
}
