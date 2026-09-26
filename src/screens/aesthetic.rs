//! Ambient: borderless full-screen scenes meant to be glanced at all day.
//! Scenes rotate on a timer (`ui.ambient_rotate_secs`); ←/→ step through
//! them and `r` toggles rotation.

use std::time::Instant;

use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Paragraph};
use ratatui::Frame;

use crate::app::App;
use crate::screens::too_small;
use crate::theme::Theme;
use crate::widgets::{clock, matrix, media, music_viz, pinned_media, upnext, video, weather};

const MIN: (u16, u16) = (60, 20);

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Scene {
    Custom(String),
    /// Giant clock over a calm audio horizon. Static when silent: the
    /// cheapest scene, and the default.
    Horizon,
    /// A big retro flip clock whose minute card folds over on the minute.
    Flip,
    /// Matrix rain with the clock floating in the middle.
    Rain,
    /// A drifting contour map; load speeds it up, music ripples it.
    Topo,
    /// The spinning donut beside the time, weather and track.
    Orbit,
    /// Now playing: art, track and a big visualizer. Only while music plays.
    Studio,
    /// The pinned image with a small clock. Only when an image is pinned.
    Gallery,
    Starfield,
    Life,
    Snow,
    Creative,
}

impl Scene {
    fn label(&self) -> &str {
        match self {
            Scene::Custom(ref id) => id.as_str(),
            Scene::Horizon => "horizon",
            Scene::Flip => "flip",
            Scene::Topo => "topography",
            Scene::Rain => "rain",
            Scene::Orbit => "orbit",
            Scene::Studio => "studio",
            Scene::Gallery => "gallery",
            Scene::Starfield => "starfield",
            Scene::Life => "life",
            Scene::Snow => "snow",
            Scene::Creative => "creative",
        }
    }
}

/// Rotation state. The scene is derived from wall time, so rendering stays
/// read-only: `index = base + elapsed / rotate_secs`.
#[derive(Clone, Debug)]
pub struct AmbientState {
    base: usize,
    anchor: Instant,
    pub auto: bool,
}

impl Default for AmbientState {
    fn default() -> Self {
        Self {
            base: 0,
            anchor: Instant::now(),
            auto: true,
        }
    }
}

impl AmbientState {
    fn index(&self, rotate_secs: u64, n: usize) -> usize {
        let steps = if self.auto && rotate_secs > 0 {
            (self.anchor.elapsed().as_secs() / rotate_secs) as usize
        } else {
            0
        };
        (self.base + steps) % n.max(1)
    }

    /// Move `delta` scenes from the one currently shown and restart the timer.
    pub fn step(&mut self, delta: isize, app_rotate_secs: u64, n: usize) {
        let n = n.max(1);
        let cur = self.index(app_rotate_secs, n) as isize;
        self.base = (cur + delta).rem_euclid(n as isize) as usize;
        self.anchor = Instant::now();
    }

    pub fn toggle_auto(&mut self, rotate_secs: u64, n: usize) {
        self.base = self.index(rotate_secs, n);
        self.anchor = Instant::now();
        self.auto = !self.auto;
    }
}

/// Scenes available right now, in rotation order.
pub fn scenes(app: &App) -> Vec<Scene> {
    let cfg = &app.config;
    let mut v = vec![Scene::Horizon, Scene::Flip];
    if cfg.widgets.matrix {
        v.push(Scene::Rain);
    }
    v.push(Scene::Topo);
    v.push(Scene::Starfield);
    v.push(Scene::Life);
    v.push(Scene::Snow);
    if cfg.widgets.video {
        v.push(Scene::Orbit);
    }
    if media::current_player().is_some() {
        v.push(Scene::Studio);
    }
    if cfg.widgets.pinned_media && !cfg.ui.pinned_media_path.is_empty() {
        v.push(Scene::Gallery);
    }
    v
}

pub fn render(f: &mut Frame, area: Rect, app: &App) {
    if area.width < MIN.0 || area.height < MIN.1 {
        too_small(f, area, &app.theme, MIN);
        return;
    }
    let theme = &app.theme;
    let list = scenes(app);
    let idx = app
        .ambient
        .index(app.config.ui.ambient_rotate_secs, list.len());
    let scene = list[idx].clone();

    // Hints overlay the bottom row only while visible, so the stage keeps
    // the full height and nothing jumps when they come and go.
    let stage = area;
    let footer = Rect::new(area.x, area.bottom() - 1, area.width, 1);
    let t = drift_step();
    match scene {
        Scene::Horizon => horizon(f, drift(stage, 3, 0, t), app, theme, t),
        Scene::Flip => flip(f, drift(stage, 3, 1, t), app, theme),
        Scene::Rain => rain(f, stage, app, theme, t),
        Scene::Topo => topo(f, stage, app, theme, t),
        Scene::Orbit => orbit(f, drift(stage, 3, 1, t), app, theme),
        Scene::Studio => studio(f, drift(stage, 3, 1, t), app, theme),
        Scene::Gallery => gallery(f, stage, app, theme, t),
        Scene::Starfield => starfield(f, stage, app, theme),
        Scene::Life => life(f, stage, app, theme),
        Scene::Snow => snow(f, stage, app, theme),
        Scene::Custom(ref id) => {
            let mut matched = false;
            for ext in &app.ext_manager.extensions {
                for mut comp in ext.components() {
                    if comp.id().eq_ignore_ascii_case(id)
                        || ext.metadata().id.eq_ignore_ascii_case(id)
                    {
                        comp.render(f, stage, theme);
                        matched = true;
                        break;
                    }
                }
                if matched {
                    break;
                }
            }
        }

        Scene::Creative => {
            let [left, right] = ratatui::layout::Layout::horizontal([ratatui::layout::Constraint::Percentage(65), ratatui::layout::Constraint::Percentage(35)]).areas(stage);
            let [video_area, weather_area] = ratatui::layout::Layout::vertical([ratatui::layout::Constraint::Fill(1), ratatui::layout::Constraint::Length(3)]).areas(left);
            let [clock_area, viz_area] = ratatui::layout::Layout::vertical([ratatui::layout::Constraint::Length(12), ratatui::layout::Constraint::Fill(1)]).areas(right);
            
            // Render native components
            crate::widgets::weather::render(f, weather_area, theme);
            big_clock(f, clock_area, app, theme);
            if app.config.widgets.music_viz {
                crate::widgets::music_viz::render(f, viz_area, theme, app.frame);
            }
            
            // Lookup and render the video_widget
            for ext in &app.ext_manager.extensions {
                for mut comp in ext.components() {
                    if comp.id() == "video_widget" {
                        comp.render(f, video_area, theme);
                    }
                }
            }
        }
    }
    if app.panel_states.pinned_media_input_active {
        f.render_widget(Clear, footer);
        f.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled("image path: ", Style::default().fg(theme.dim)),
                Span::styled(
                    format!("{}_", app.panel_states.pinned_media_input),
                    Style::default().fg(theme.accent),
                ),
            ]))
            .alignment(Alignment::Center),
            footer,
        );
    } else if app.last_input.elapsed() < HINT_SECS {
        f.render_widget(Clear, footer);
        render_footer(f, footer, theme, &list, idx, app.ambient.auto);
    }
}

/// Scene hints disappear this long after the last key press.
const HINT_SECS: std::time::Duration = std::time::Duration::from_secs(10);
/// Content shifts one step this often, slowly enough to go unnoticed.
const DRIFT_SECS: u64 = 90;

fn drift_step() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() / DRIFT_SECS)
}

/// Triangle wave over 0..=amp.
fn tri(t: u64, amp: u16) -> u16 {
    if amp == 0 {
        return 0;
    }
    let period = 2 * amp as u64;
    let k = t % period;
    (if k <= amp as u64 { k } else { period - k }) as u16
}

/// Burn-in protection: `area` shrunk by `ax`/`ay` on each side and nudged
/// around the freed margin, so static glyphs (the clock above all) never
/// sit on the same cells for hours on OLED/plasma screens. The vertical
/// axis moves slower so the path wanders instead of tracing one diagonal.
fn drift(area: Rect, ax: u16, ay: u16, t: u64) -> Rect {
    let ax = ax.min(area.width / 8);
    let ay = ay.min(area.height / 8);
    Rect::new(
        area.x + tri(t, 2 * ax),
        area.y + tri(t / 5, 2 * ay),
        area.width - 2 * ax,
        area.height - 2 * ay,
    )
}

fn big_clock(f: &mut Frame, area: Rect, app: &App, theme: &Theme) {
    let note = upnext::next_note();
    clock::render_with_note(
        f,
        area,
        theme,
        app.config.ui.clock_24h,
        &app.config.ui.clock_font,
        &app.config.ui.clock_style,
        &[],
        note.as_deref(),
    );
}

/// Centered single line of weather under the clock.
fn weather_line(f: &mut Frame, area: Rect, theme: &Theme) {
    if !crate::monitors::weather::snapshot().ready || area.height == 0 {
        return;
    }
    let w = area.width.min(48);
    weather::render(
        f,
        Rect::new(area.x + (area.width - w) / 2, area.y, w, 1),
        theme,
    );
}

/// The audio horizon stays pinned to the bottom edge; only the clock
/// block above it drifts vertically.
fn horizon(f: &mut Frame, area: Rect, app: &App, theme: &Theme, t: u64) {
    let viz_h = (area.height / 4).clamp(4, 10);
    let [sky, viz] = Layout::vertical([Constraint::Min(0), Constraint::Length(viz_h)]).areas(area);
    let sky = drift(sky, 0, 1, t);
    let [_, clock_area, wx, _] = Layout::vertical([
        Constraint::Fill(1),
        Constraint::Length((sky.height.saturating_sub(4)).min(18)),
        Constraint::Length(1),
        Constraint::Fill(1),
    ])
    .areas(sky);
    // Side margins keep the glyphs from running edge to edge.
    let margin = clock_area.width / 8;
    big_clock(
        f,
        Rect::new(
            clock_area.x + margin,
            clock_area.y,
            clock_area.width - 2 * margin,
            clock_area.height,
        ),
        app,
        theme,
    );
    weather_line(f, wx, theme);
    if app.config.widgets.music_viz {
        music_viz::render(f, viz, theme, app.frame);
    }
}

fn flip(f: &mut Frame, area: Rect, app: &App, theme: &Theme) {
    let transparent = app
        .config
        .ui
        .transparent
        .unwrap_or_else(|| !theme.is_light());
    let note = upnext::next_note();
    crate::widgets::flip_clock::render(
        f,
        area,
        theme,
        app.config.ui.clock_24h,
        transparent,
        note.as_deref(),
    );
}

/// Full-bleed map with a small legend that wanders along the bottom-left
/// corner (the map itself is always moving, so only the legend needs it).
fn topo(f: &mut Frame, area: Rect, app: &App, theme: &Theme, t: u64) {
    crate::widgets::topo::render(
        f,
        area,
        theme,
        app.summary.cpu_pct,
        app.config.ui.motion_enabled,
        app.night == Some(true),
    );
    let time = if app.config.ui.clock_24h {
        chrono::Local::now().format("%H:%M").to_string()
    } else {
        chrono::Local::now().format("%-I:%M %P").to_string()
    };
    let mut legend = format!(" {}  ·  cpu {:.0}% ", time, app.summary.cpu_pct);
    if crate::widgets::topo::reacting_to_music() {
        legend.push_str("·  ♪ ");
    }
    let w = (legend.chars().count() as u16).min(area.width);
    let (x, y) = (
        area.x + 1 + tri(t, 6).min(area.width.saturating_sub(w + 1)),
        area.bottom().saturating_sub(3 + tri(t / 5, 2)),
    );
    f.render_widget(Clear, Rect::new(x, y, w, 1));
    f.render_widget(
        Paragraph::new(Span::styled(legend, Style::default().fg(theme.dim))),
        Rect::new(x, y, w, 1),
    );
}

fn rain(f: &mut Frame, area: Rect, app: &App, theme: &Theme, t: u64) {
    matrix::render(f, area, theme);
    let w = (area.width * 3 / 5).clamp(40, 90).min(area.width);
    let h = 12.min(area.height);
    // The card floats around the middle half of the free space.
    let (fx, fy) = ((area.width - w) / 2, (area.height - h) / 2);
    let card = Rect::new(
        area.x + fx / 2 + tri(t, fx),
        area.y + fy / 2 + tri(t / 5, fy),
        w,
        h,
    );
    f.render_widget(Clear, card);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.surface))
        .style(Style::default().bg(theme.bg));
    let inner = block.inner(card);
    f.render_widget(block, card);
    big_clock(f, inner, app, theme);
}

fn orbit(f: &mut Frame, area: Rect, app: &App, theme: &Theme) {
    let [left, right] =
        Layout::horizontal([Constraint::Percentage(55), Constraint::Percentage(45)]).areas(area);
    video::render_with_motion(
        f,
        left,
        theme,
        app.frame,
        app.config.ui.motion_enabled,
        app.config.ui.motion_speed,
        &app.config.ui.motion_mode,
    );
    let [_, clk, wx, _, track, _] = Layout::vertical([
        Constraint::Fill(1),
        Constraint::Length(10),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(3),
        Constraint::Fill(1),
    ])
    .areas(right);
    big_clock(f, clk, app, theme);
    weather_line(f, wx, theme);
    if media::current_player().is_some() {
        media::render(f, track, theme);
    }
}

fn studio(f: &mut Frame, area: Rect, app: &App, theme: &Theme) {
    let [_, info, _, viz] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length((area.height / 3).clamp(4, 10)),
        Constraint::Length(1),
        Constraint::Min(4),
    ])
    .areas(area);
    let pad = area.width / 10;
    media::render(
        f,
        Rect::new(info.x + pad, info.y, info.width - 2 * pad, info.height),
        theme,
    );
    music_viz::render(f, viz, theme, app.frame);
}

fn gallery(f: &mut Frame, area: Rect, app: &App, theme: &Theme, t: u64) {
    pinned_media::render(f, area, theme, &app.config.ui.pinned_media_path, app.frame);
    let time = if app.config.ui.clock_24h {
        chrono::Local::now().format(" %H:%M ").to_string()
    } else {
        chrono::Local::now().format(" %-I:%M %P ").to_string()
    };
    let w = time.chars().count() as u16;
    f.render_widget(
        Paragraph::new(Span::styled(
            time,
            Style::default()
                .fg(theme.text)
                .bg(theme.bg)
                .add_modifier(Modifier::BOLD),
        )),
        // Wanders along the top-right corner like the other scenes' clocks.
        Rect::new(
            area.x + area.width.saturating_sub(w + 1 + tri(t, 6)),
            area.y + tri(t / 5, 2).min(area.height.saturating_sub(1)),
            w.min(area.width),
            1,
        ),
    );
}

/// Scene dots, e.g. "○ ● ○ ○  rain · ←/→ · r pause".
fn render_footer(f: &mut Frame, area: Rect, theme: &Theme, list: &[Scene], idx: usize, auto: bool) {
    let mut spans: Vec<Span> = list
        .iter()
        .enumerate()
        .map(|(i, _)| {
            if i == idx {
                Span::styled("● ", Style::default().fg(theme.accent))
            } else {
                Span::styled("○ ", Style::default().fg(theme.surface))
            }
        })
        .collect();
    spans.push(Span::styled(
        format!(
            " {} · ←/→ scenes · r {} · i image",
            list[idx].label(),
            if auto { "pause" } else { "rotate" }
        ),
        Style::default().fg(theme.dim),
    ));
    f.render_widget(
        Paragraph::new(Line::from(spans)).alignment(Alignment::Center),
        area,
    );
}

fn starfield(f: &mut Frame, area: Rect, _app: &App, theme: &Theme) {
    crate::widgets::starfield::render(f, area, theme);
}

fn life(f: &mut Frame, area: Rect, _app: &App, theme: &Theme) {
    crate::widgets::life::render(f, area, theme);
}

fn snow(f: &mut Frame, area: Rect, _app: &App, theme: &Theme) {
    crate::widgets::snow::render(f, area, theme);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stepping_wraps_and_pausing_freezes_the_scene() {
        let mut s = AmbientState::default();
        assert_eq!(s.index(300, 3), 0);
        s.step(-1, 300, 3);
        assert_eq!(s.index(300, 3), 2);
        s.step(1, 300, 3);
        assert_eq!(s.index(300, 3), 0);
        s.toggle_auto(300, 3);
        assert!(!s.auto);
        assert_eq!(s.index(1, 3), 0, "paused state ignores elapsed time");
    }

    #[test]
    fn drift_stays_inside_the_stage_and_visits_every_offset() {
        let stage = Rect::new(0, 1, 120, 30);
        let mut xs = std::collections::BTreeSet::new();
        let mut ys = std::collections::BTreeSet::new();
        for t in 0..200 {
            let r = drift(stage, 3, 1, t);
            assert!(r.x >= stage.x && r.right() <= stage.right());
            assert!(r.y >= stage.y && r.bottom() <= stage.bottom());
            assert_eq!((r.width, r.height), (114, 28));
            xs.insert(r.x);
            ys.insert(r.y);
        }
        assert_eq!(xs.len(), 7);
        assert_eq!(ys.len(), 3);
        // Tiny areas don't drift (and don't underflow).
        assert_eq!(drift(Rect::new(0, 0, 7, 7), 3, 1, 5), Rect::new(0, 0, 7, 7));
    }
}
