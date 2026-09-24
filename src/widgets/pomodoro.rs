//! Pomodoro focus timer: focus → short break, with a long break after every
//! fourth focus session. Global state so the title bar can show it on any page.

use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant};

use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::config::UiConfig;
use crate::theme::Theme;
use crate::widgets::{clock, meter};

const SESSIONS_PER_LONG_BREAK: u32 = 4;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Focus,
    ShortBreak,
    LongBreak,
}

impl Phase {
    pub fn label(self) -> &'static str {
        match self {
            Phase::Focus => "focus",
            Phase::ShortBreak => "break",
            Phase::LongBreak => "long break",
        }
    }

    pub fn color(self, theme: &Theme) -> Color {
        match self {
            Phase::Focus => theme.accent,
            Phase::ShortBreak => theme.green,
            Phase::LongBreak => theme.secondary,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Durations {
    pub focus: Duration,
    pub short: Duration,
    pub long: Duration,
}

impl Durations {
    pub fn from_config(ui: &UiConfig) -> Self {
        let m = |v: u64| Duration::from_secs(v.clamp(1, 240) * 60);
        Self {
            focus: m(ui.focus_minutes),
            short: m(ui.break_minutes),
            long: m(ui.long_break_minutes),
        }
    }

    fn of(&self, p: Phase) -> Duration {
        match p {
            Phase::Focus => self.focus,
            Phase::ShortBreak => self.short,
            Phase::LongBreak => self.long,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Timer {
    pub phase: Phase,
    /// Time left while paused; ignored while running.
    left: Duration,
    /// Set while running.
    ends_at: Option<Instant>,
    /// Focus sessions completed since the last long break.
    pub done: u32,
    /// Whether the current phase has been started at least once.
    pub started: bool,
}

impl Timer {
    pub fn new(d: &Durations) -> Self {
        Self {
            phase: Phase::Focus,
            left: d.focus,
            ends_at: None,
            done: 0,
            started: false,
        }
    }

    pub fn running(&self) -> bool {
        self.ends_at.is_some()
    }

    pub fn remaining(&self, now: Instant) -> Duration {
        match self.ends_at {
            Some(end) => end.saturating_duration_since(now),
            None => self.left,
        }
    }

    pub fn toggle(&mut self, now: Instant) {
        match self.ends_at.take() {
            Some(end) => self.left = end.saturating_duration_since(now),
            None => {
                self.ends_at = Some(now + self.left);
                self.started = true;
            }
        }
    }

    /// Restart the current phase from its full length, paused.
    pub fn reset(&mut self, d: &Durations) {
        self.ends_at = None;
        self.left = d.of(self.phase);
        self.started = false;
    }

    /// Move to the next phase, paused. Completing (or skipping) a focus
    /// session counts toward the long break.
    pub fn advance(&mut self, d: &Durations) {
        self.phase = match self.phase {
            Phase::Focus => {
                self.done += 1;
                if self.done >= SESSIONS_PER_LONG_BREAK {
                    Phase::LongBreak
                } else {
                    Phase::ShortBreak
                }
            }
            Phase::LongBreak => {
                self.done = 0;
                Phase::Focus
            }
            Phase::ShortBreak => Phase::Focus,
        };
        self.reset(d);
    }

    /// Returns the phase that just finished, if the running timer hit zero.
    pub fn tick(&mut self, now: Instant, d: &Durations) -> Option<Phase> {
        if self.ends_at.is_some_and(|end| now >= end) {
            let finished = self.phase;
            self.advance(d);
            return Some(finished);
        }
        None
    }

    /// Fraction of the current phase elapsed.
    pub fn progress(&self, now: Instant, d: &Durations) -> f64 {
        let total = d.of(self.phase).as_secs_f64().max(1.0);
        1.0 - self.remaining(now).as_secs_f64() / total
    }
}

static TIMER: LazyLock<Mutex<Option<Timer>>> = LazyLock::new(|| Mutex::new(None));

fn with<R>(ui: &UiConfig, f: impl FnOnce(&mut Timer, &Durations) -> R) -> R {
    let d = Durations::from_config(ui);
    let mut g = TIMER.lock().unwrap();
    let t = g.get_or_insert_with(|| Timer::new(&d));
    f(t, &d)
}

pub fn toggle(ui: &UiConfig) {
    with(ui, |t, _| t.toggle(Instant::now()));
}

pub fn reset(ui: &UiConfig) {
    with(ui, |t, d| t.reset(d));
}

pub fn skip(ui: &UiConfig) {
    with(ui, |t, d| t.advance(d));
}

/// Advance the timer; call once per frame. Notifies on phase completion.
pub fn tick(ui: &UiConfig) {
    let finished = with(ui, |t, d| t.tick(Instant::now(), d));
    if let Some(phase) = finished {
        let body = match phase {
            Phase::Focus => "Focus session done. Take a break.",
            _ => "Break's over. Back to it.",
        };
        // Desktop notification when available; the terminal bell otherwise.
        if std::process::Command::new("notify-send")
            .args(["-a", "vanta", "vanta", body])
            .spawn()
            .is_err()
        {
            use std::io::Write;
            let _ = std::io::stdout().write_all(b"\x07");
        }
    }
}

fn mmss(d: Duration) -> String {
    let s = d.as_secs() + u64::from(d.subsec_nanos() > 0);
    format!("{:02}:{:02}", s / 60, s % 60)
}

/// Compact status for the title bar, e.g. ("focus", "18:32", color), while
/// a session is running or paused mid-way.
pub fn badge(theme: &Theme) -> Option<(String, Color)> {
    let g = TIMER.lock().unwrap();
    let t = g.as_ref().filter(|t| t.started)?;
    let icon = if t.running() { "◷" } else { "⏸" };
    Some((
        format!(
            "{} {} {}",
            icon,
            t.phase.label(),
            mmss(t.remaining(Instant::now()))
        ),
        t.phase.color(theme),
    ))
}

pub fn render(f: &mut Frame, area: Rect, theme: &Theme, ui: &UiConfig, focused: bool) {
    if area.height == 0 || area.width < 16 {
        return;
    }
    let now = Instant::now();
    let (phase, running, started, left, frac, done) = with(ui, |t, d| {
        (
            t.phase,
            t.running(),
            t.started,
            t.remaining(now),
            t.progress(now, d),
            t.done,
        )
    });
    let color = phase.color(theme);
    let digits = if running || !started {
        color
    } else {
        theme.dim
    };

    let state = match (running, started) {
        (true, _) => "",
        (false, true) => " · paused",
        (false, false) => " · ready",
    };
    let session = (done + u32::from(phase == Phase::Focus)).min(SESSIONS_PER_LONG_BREAK);
    let head = Line::from(vec![
        Span::styled(
            phase.label().to_uppercase(),
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!(
                "{}  ·  session {}/{}",
                state,
                session.max(1),
                SESSIONS_PER_LONG_BREAK
            ),
            Style::default().fg(theme.dim),
        ),
    ]);

    if area.height < 4 {
        let line = Line::from(vec![
            Span::styled(format!("{} ", mmss(left)), Style::default().fg(digits)),
            Span::styled(
                format!("{}{}", phase.label(), state),
                Style::default().fg(theme.dim),
            ),
        ]);
        f.render_widget(Paragraph::new(line).alignment(Alignment::Center), area);
        return;
    }

    let [h, big, bar, dots] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(1),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .areas(area);
    f.render_widget(Paragraph::new(head).alignment(Alignment::Center), h);
    if clock::render_big_text(f, big, &mmss(left), digits, "standard", "solid") == 0 {
        f.render_widget(
            Paragraph::new(Span::styled(mmss(left), Style::default().fg(digits)))
                .alignment(Alignment::Center),
            Rect::new(big.x, big.y + big.height / 2, big.width, 1),
        );
    }
    let bar_w = (bar.width as usize).saturating_sub(4).min(48);
    let (on, off) = meter::track(frac, bar_w);
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(on, Style::default().fg(color)),
            Span::styled(off, Style::default().fg(theme.surface)),
        ]))
        .alignment(Alignment::Center),
        bar,
    );

    let mut spans: Vec<Span> = (0..SESSIONS_PER_LONG_BREAK)
        .map(|i| {
            if i < done {
                Span::styled("● ", Style::default().fg(theme.accent))
            } else {
                Span::styled("○ ", Style::default().fg(theme.surface))
            }
        })
        .collect();
    let verb = if running { "pause" } else { "start" };
    let room = (dots.width as usize).saturating_sub(SESSIONS_PER_LONG_BREAK as usize * 2);
    let hint = [
        format!("  space {} · r reset · s skip", verb),
        format!("  space {} · r · s", verb),
        format!("  space {}", verb),
    ]
    .into_iter()
    .find(|h| h.chars().count() <= room)
    .unwrap_or_default();
    let hint = if focused { hint } else { String::new() };
    spans.push(Span::styled(hint, Style::default().fg(theme.dim)));
    f.render_widget(
        Paragraph::new(Line::from(spans)).alignment(Alignment::Center),
        dots,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d() -> Durations {
        Durations {
            focus: Duration::from_secs(25 * 60),
            short: Duration::from_secs(5 * 60),
            long: Duration::from_secs(15 * 60),
        }
    }

    #[test]
    fn pause_resume_keeps_remaining_time() {
        let d = d();
        let mut t = Timer::new(&d);
        let t0 = Instant::now();
        t.toggle(t0);
        assert!(t.running());
        t.toggle(t0 + Duration::from_secs(60));
        assert_eq!(
            t.remaining(t0 + Duration::from_secs(600)),
            Duration::from_secs(24 * 60)
        );
        t.toggle(t0 + Duration::from_secs(600));
        assert_eq!(
            t.remaining(t0 + Duration::from_secs(660)),
            Duration::from_secs(23 * 60)
        );
    }

    #[test]
    fn completes_into_breaks_and_long_break_after_four() {
        let d = d();
        let mut t = Timer::new(&d);
        let mut now = Instant::now();
        let mut phases = Vec::new();
        for _ in 0..8 {
            t.toggle(now);
            now += d.of(t.phase);
            phases.push(t.tick(now, &d).unwrap());
        }
        use Phase::*;
        assert_eq!(
            phases,
            [Focus, ShortBreak, Focus, ShortBreak, Focus, ShortBreak, Focus, LongBreak]
        );
        assert_eq!(t.phase, Focus);
        assert_eq!(t.done, 0);
        assert!(!t.running(), "each phase waits to be started");
    }

    #[test]
    fn formats_minutes_rounding_partial_seconds_up() {
        assert_eq!(mmss(Duration::from_millis(59_001)), "01:00");
        assert_eq!(mmss(Duration::from_secs(25 * 60)), "25:00");
        assert_eq!(mmss(Duration::ZERO), "00:00");
    }
}
