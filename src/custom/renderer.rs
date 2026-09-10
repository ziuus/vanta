//! Rendering logic for custom widgets.
//!
//! Each renderer is a pure function that takes an inner `Rect`, the widget's
//! cached data, its config, and the active `Theme`, then draws into a ratatui
//! `Frame`.  No I/O, no blocking.

use std::collections::VecDeque;

use ratatui::layout::Alignment;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::custom::config::{CustomWidgetConfig, RendererKind};
use crate::custom::source::FetchResult;
use crate::theme::Theme;
use crate::widgets::block_graph::BlockGraph;
use crate::widgets::meter::bar;

// ── Numeric parsing ───────────────────────────────────────────────────────────

/// Parse a raw string into a numeric value.
///
/// Handles:
/// - plain integers: `"82"`
/// - plain floats:   `"12.4"`
/// - percentage:     `"82%"`  → 82.0
/// - unit-suffixed:  `"12.4 V"`, `"42000"`, `"1.2GHz"` → leading number
///
/// Returns `None` when no numeric prefix can be found.
pub fn parse_numeric(s: &str) -> Option<f64> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    // Strip a trailing `%` and try.
    let stripped = s.strip_suffix('%').unwrap_or(s);
    // Take the longest leading numeric prefix (digits, dot, sign, exponent).
    let end = stripped
        .char_indices()
        .take_while(|(_, c)| c.is_ascii_digit() || *c == '.' || *c == '-' || *c == '+')
        .map(|(i, c)| i + c.len_utf8())
        .last()
        .unwrap_or(0);
    if end == 0 {
        return None;
    }
    stripped[..end].parse::<f64>().ok()
}

/// Clamp and normalise `value` to 0.0..=1.0 using `min`/`max`.
pub fn normalise(value: f64, min: f64, max: f64) -> f64 {
    let span = (max - min).max(f64::EPSILON);
    ((value - min) / span).clamp(0.0, 1.0)
}

// ── Top-level dispatcher ──────────────────────────────────────────────────────

/// Render a custom widget's inner area given its current `FetchResult` and
/// historic `history` buffer.
pub fn render(
    f: &mut Frame,
    area: Rect,
    cfg: &CustomWidgetConfig,
    result: &FetchResult,
    history: &VecDeque<f64>,
    theme: &Theme,
) {
    // All renderers start by checking for error states.
    if let Some(label) = error_label(result) {
        render_status_line(f, area, label, theme);
        return;
    }
    let raw = match result {
        FetchResult::Ok(s) => s.as_str(),
        _ => "",
    };

    match cfg.renderer {
        RendererKind::Value => render_value(f, area, raw, cfg, theme),
        RendererKind::Text => render_text(f, area, raw, theme),
        RendererKind::Gauge => render_gauge(f, area, raw, cfg, theme),
        RendererKind::Bar => render_bar(f, area, raw, cfg, theme),
        RendererKind::Graph => render_graph(f, area, history, cfg, theme),
    }
}

fn error_label(result: &FetchResult) -> Option<&str> {
    match result {
        FetchResult::Ok(_) => None,
        other => Some(other.error_label()),
    }
}

// ── Status / error line ───────────────────────────────────────────────────────

fn render_status_line(f: &mut Frame, area: Rect, msg: &str, theme: &Theme) {
    if area.height == 0 {
        return;
    }
    let y = area.y + area.height / 2;
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            msg.to_string(),
            Style::default().fg(theme.dim),
        )))
        .alignment(Alignment::Center),
        Rect::new(area.x, y, area.width, 1),
    );
}

// ── Value renderer ────────────────────────────────────────────────────────────

/// Display the value (with optional unit) large and centred.
///
/// ```text
/// ┌ Pi Battery ──────────┐
/// │                      │
/// │      12.4 V          │
/// │                      │
/// └──────────────────────┘
/// ```
fn render_value(f: &mut Frame, area: Rect, raw: &str, cfg: &CustomWidgetConfig, theme: &Theme) {
    if area.height == 0 {
        return;
    }
    let display = format_with_unit(raw, cfg);
    let y = area.y + area.height / 2;
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            display,
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        )))
        .alignment(Alignment::Center),
        Rect::new(area.x, y, area.width, 1),
    );
}

// ── Text renderer ─────────────────────────────────────────────────────────────

/// Display the raw text output, word-wrapped.
fn render_text(f: &mut Frame, area: Rect, raw: &str, theme: &Theme) {
    if area.height == 0 || area.width == 0 {
        return;
    }
    let lines: Vec<Line> = raw
        .lines()
        .take(area.height as usize)
        .map(|l| {
            Line::from(Span::styled(
                crate::widgets::meter::ellipsize(l, area.width as usize),
                Style::default().fg(theme.text),
            ))
        })
        .collect();
    f.render_widget(Paragraph::new(lines), area);
}

// ── Gauge renderer ────────────────────────────────────────────────────────────

/// Horizontal progress bar.
///
/// ```text
/// ████████████░░ 82%
/// ```
fn render_gauge(f: &mut Frame, area: Rect, raw: &str, cfg: &CustomWidgetConfig, theme: &Theme) {
    if area.height == 0 || area.width < 6 {
        return;
    }
    let Some(value) = parse_numeric(raw) else {
        render_status_line(f, area, "Invalid value", theme);
        return;
    };
    let frac = normalise(value, cfg.effective_min(), cfg.effective_max());
    let pct = frac * 100.0;

    // Choose colour the same way theme.usage() does so it's consistent.
    let col = theme.usage(pct);

    // Bar takes most of the width; reserve some for the percentage label.
    let label = format!("{:.0}%", pct);
    let label_w = label.len() + 1; // space + label
    let bar_w = (area.width as usize).saturating_sub(label_w).max(1);

    let filled = bar(frac, bar_w);
    let y = area.y + area.height / 2;
    let spans = vec![
        Span::styled(filled, Style::default().fg(col)),
        Span::styled(format!(" {}", label), Style::default().fg(theme.text)),
    ];
    f.render_widget(
        Paragraph::new(Line::from(spans)),
        Rect::new(area.x, y, area.width, 1),
    );
}

// ── Bar renderer ─────────────────────────────────────────────────────────────

/// Simple single horizontal bar — same visual as gauge but without the label.
fn render_bar(f: &mut Frame, area: Rect, raw: &str, cfg: &CustomWidgetConfig, theme: &Theme) {
    if area.height == 0 || area.width == 0 {
        return;
    }
    let Some(value) = parse_numeric(raw) else {
        render_status_line(f, area, "Invalid value", theme);
        return;
    };
    let frac = normalise(value, cfg.effective_min(), cfg.effective_max());
    let col = theme.usage(frac * 100.0);
    let filled = bar(frac, area.width as usize);
    let y = area.y + area.height / 2;
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(filled, Style::default().fg(col)))),
        Rect::new(area.x, y, area.width, 1),
    );
}

// ── Graph renderer ────────────────────────────────────────────────────────────

/// Scrolling history graph using the existing `BlockGraph` widget.
fn render_graph(
    f: &mut Frame,
    area: Rect,
    history: &VecDeque<f64>,
    cfg: &CustomWidgetConfig,
    theme: &Theme,
) {
    if area.height == 0 || area.width == 0 {
        return;
    }
    let data: Vec<f64> = history.iter().copied().collect();
    let max = cfg.effective_max();
    f.render_widget(
        BlockGraph::new(&data)
            .max(max)
            .colors(theme.accent, theme.yellow, theme.red),
        area,
    );
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Append the configured unit to the raw value if the unit is set and the raw
/// value doesn't already end with it.
fn format_with_unit(raw: &str, cfg: &CustomWidgetConfig) -> String {
    match &cfg.unit {
        Some(u) if !raw.ends_with(u.as_str()) => format!("{} {}", raw, u),
        _ => raw.to_string(),
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ---- parse_numeric -------------------------------------------------------

    #[test]
    fn parse_plain_integer() {
        assert_eq!(parse_numeric("82"), Some(82.0));
    }

    #[test]
    fn parse_plain_float() {
        assert_eq!(parse_numeric("12.4"), Some(12.4));
    }

    #[test]
    fn parse_percentage() {
        assert_eq!(parse_numeric("82%"), Some(82.0));
    }

    #[test]
    fn parse_value_with_unit_suffix() {
        // "12.4 V" — the numeric prefix is "12.4".
        assert_eq!(parse_numeric("12.4 V"), Some(12.4));
    }

    #[test]
    fn parse_large_integer() {
        assert_eq!(parse_numeric("42000"), Some(42000.0));
    }

    #[test]
    fn parse_millidegree_temp() {
        // /sys/class/thermal/thermal_zone0/temp returns values like "56000"
        assert_eq!(parse_numeric("56000"), Some(56000.0));
    }

    #[test]
    fn parse_attached_unit_no_space() {
        // "1.2GHz" — numeric prefix "1.2".
        assert_eq!(parse_numeric("1.2GHz"), Some(1.2));
    }

    #[test]
    fn parse_empty_returns_none() {
        assert_eq!(parse_numeric(""), None);
        assert_eq!(parse_numeric("   "), None);
    }

    #[test]
    fn parse_pure_text_returns_none() {
        assert_eq!(parse_numeric("Online"), None);
        assert_eq!(parse_numeric("N/A"), None);
    }

    // ---- normalise ----------------------------------------------------------

    #[test]
    fn normalise_midpoint() {
        assert!((normalise(50.0, 0.0, 100.0) - 0.5).abs() < 1e-9);
    }

    #[test]
    fn normalise_clamps_below_min() {
        assert_eq!(normalise(-10.0, 0.0, 100.0), 0.0);
    }

    #[test]
    fn normalise_clamps_above_max() {
        assert_eq!(normalise(150.0, 0.0, 100.0), 1.0);
    }

    #[test]
    fn normalise_custom_range() {
        // value=75 in [50, 100] → 50% of the way
        assert!((normalise(75.0, 50.0, 100.0) - 0.5).abs() < 1e-9);
    }

    #[test]
    fn normalise_degenerate_range_does_not_panic() {
        // min == max; span is clamped to EPSILON, result is clamped to 1.0
        let r = normalise(5.0, 5.0, 5.0);
        assert!((0.0..=1.0).contains(&r));
    }

    // ---- format_with_unit --------------------------------------------------

    #[test]
    fn unit_appended_when_absent() {
        let cfg = CustomWidgetConfig {
            unit: Some("V".into()),
            ..Default::default()
        };
        assert_eq!(format_with_unit("12.4", &cfg), "12.4 V");
    }

    #[test]
    fn unit_not_duplicated() {
        let cfg = CustomWidgetConfig {
            unit: Some("V".into()),
            ..Default::default()
        };
        assert_eq!(format_with_unit("12.4 V", &cfg), "12.4 V");
    }

    #[test]
    fn no_unit_returns_raw() {
        let cfg = CustomWidgetConfig::default();
        assert_eq!(format_with_unit("Online", &cfg), "Online");
    }

    // ---- gauge normalisation with min/max ----------------------------------

    #[test]
    fn gauge_normalises_temperature() {
        // /sys temp returns millidegrees; user sets min=0, max=100000
        let cfg = CustomWidgetConfig {
            min: Some(0.0),
            max: Some(100_000.0),
            ..Default::default()
        };
        let frac = normalise(56_000.0, cfg.effective_min(), cfg.effective_max());
        assert!((frac - 0.56).abs() < 1e-9);
    }

    // ---- bounded graph history ---------------------------------------------

    #[test]
    fn vecdeque_bounded_at_capacity() {
        const CAP: usize = 5;
        let mut hist: VecDeque<f64> = VecDeque::with_capacity(CAP);
        for i in 0..10 {
            if hist.len() >= CAP {
                hist.pop_front();
            }
            hist.push_back(i as f64);
        }
        assert_eq!(hist.len(), CAP);
        assert_eq!(*hist.front().unwrap(), 5.0);
        assert_eq!(*hist.back().unwrap(), 9.0);
    }
}
