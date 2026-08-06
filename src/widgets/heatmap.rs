use std::sync::{LazyLock, Mutex};

use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::app::Theme;

/// Columns of per-core history kept for the heatmap. One column per sample.
const HIST: usize = 240;
/// Cores we're willing to track. Rows beyond this are ignored.
const MAX_CORES: usize = 64;

struct CoreHist {
    /// `grid[core][slot]` = usage percent at that slot.
    grid: Vec<[f32; HIST]>,
    idx: usize,
    fill: usize,
    /// Last sample, so we only advance when the values actually change
    /// (render runs ~125fps but the sampler refreshes far slower).
    last: Vec<f32>,
}

static HEAT: LazyLock<Mutex<CoreHist>> = LazyLock::new(|| {
    Mutex::new(CoreHist {
        grid: Vec::new(),
        idx: 0,
        fill: 0,
        last: Vec::new(),
    })
});

/// Record the current per-core usage, returning (grid, idx, fill).
fn record(cores: &[f32]) -> (Vec<[f32; HIST]>, usize, usize) {
    let mut h = HEAT.lock().unwrap();
    let n = cores.len().min(MAX_CORES);

    if h.grid.len() != n {
        h.grid = vec![[0.0; HIST]; n];
        h.last = vec![-1.0; n];
        h.idx = 0;
        h.fill = 0;
    }

    // sysinfo reports identical values until it refreshes; skipping those keeps
    // one column per real sample instead of 125 duplicates per second.
    if h.last[..n] != cores[..n] {
        let pos = h.idx;
        for (c, &v) in cores.iter().take(n).enumerate() {
            h.grid[c][pos] = v;
        }
        h.last[..n].copy_from_slice(&cores[..n]);
        h.idx = (pos + 1) % HIST;
        h.fill = (h.fill + 1).min(HIST);
    }

    (h.grid.clone(), h.idx, h.fill)
}

/// Heat ramp: dim → accent → yellow → red as load climbs.
/// Uses half-block shading so each cell carries two intensity steps.
fn heat_cell(pct: f32, theme: &Theme) -> (char, Color) {
    let p = pct.clamp(0.0, 100.0);
    let col = if p >= 85.0 {
        theme.red
    } else if p >= 60.0 {
        theme.yellow
    } else if p >= 30.0 {
        theme.accent
    } else if p >= 10.0 {
        theme.green
    } else {
        theme.surface
    };
    // Shade density within the band gives sub-threshold resolution.
    let ch = match p as u32 {
        0..=4 => ' ',
        5..=19 => '░',
        20..=49 => '▒',
        50..=79 => '▓',
        _ => '█',
    };
    (ch, col)
}

/// Per-core × time heatmap. Each row is one core, each column one sample —
/// so a single pinned core shows as a bright horizontal streak that neither
/// the aggregate graph nor instantaneous bars can reveal.
pub fn render(f: &mut Frame, area: Rect, theme: &Theme) {
    if area.height < 3 || area.width < 24 {
        return;
    }

    let cores: Vec<f32> = {
        let sys = crate::app::SYS.lock().unwrap();
        sys.cpus().iter().map(|c| c.cpu_usage()).collect()
    };
    if cores.is_empty() {
        return;
    }

    let (grid, idx, fill) = record(&cores);
    let n = grid.len();
    if n == 0 {
        return;
    }

    // label_w: e.g. "c12 " (3 or 4)
    let label_w = if n >= 10 { 3 } else { 2 };
    let bar_w = 6;
    // "c0 " + bar + " 100% " = label_w + 1 + bar_w + 6 = label_w + bar_w + 7
    let prefix_w = label_w + 1 + bar_w + 6;
    let graph_w = (area.width as usize).saturating_sub(prefix_w);

    let rows = n.min(area.height as usize);
    let want = graph_w.min(fill);

    let mut lines: Vec<Line<'static>> = Vec::with_capacity(rows);
    for (c, hist) in grid.iter().enumerate().take(rows) {
        let mut spans: Vec<Span<'static>> = Vec::with_capacity(want + 4);
        let usage = cores.get(c).copied().unwrap_or(0.0);
        let bar_col = if usage >= 90.0 {
            theme.red
        } else if usage >= 70.0 {
            theme.yellow
        } else {
            theme.accent
        };

        spans.push(Span::styled(
            format!("c{:<w$} ", c, w = label_w - 1),
            Style::default().fg(theme.dim),
        ));

        let filled = ((usage / 100.0).clamp(0.0, 1.0) * bar_w as f32).round() as usize;
        spans.push(Span::styled(
            format!("{}{}", "━".repeat(filled), "─".repeat(bar_w - filled)),
            Style::default().fg(bar_col),
        ));

        spans.push(Span::styled(
            format!("{:>4.0}% ", usage),
            Style::default()
                .fg(bar_col)
                .add_modifier(ratatui::style::Modifier::BOLD),
        ));

        if want < graph_w {
            spans.push(Span::raw(" ".repeat(graph_w - want)));
        }
        for i in 0..want {
            let slot = (idx + HIST - want + i) % HIST;
            let p = hist[slot];
            let (ch, col) = heat_cell(p, theme);
            spans.push(Span::styled(ch.to_string(), Style::default().fg(col)));
        }

        lines.push(Line::from(spans));
    }

    let top = (area.height.saturating_sub(lines.len() as u16)) / 2;
    f.render_widget(
        Paragraph::new(lines),
        Rect::new(area.x, area.y + top, area.width, area.height - top),
    );
}
