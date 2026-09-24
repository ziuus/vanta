use std::sync::{LazyLock, Mutex};

use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::monitors::history::History;
use crate::theme::Theme;
use crate::widgets::block_graph::BlockGraph;
use crate::widgets::meter;

#[derive(Clone, Copy, Default)]
pub struct MemSnapshot {
    pub total: u64,
    pub used: u64,
    pub swap_total: u64,
    pub swap_used: u64,
    pub available: u64,
    pub free: u64,
}

impl MemSnapshot {
    pub fn pct(&self) -> f64 {
        if self.total == 0 {
            0.0
        } else {
            self.used as f64 / self.total as f64 * 100.0
        }
    }
    pub fn swap_pct(&self) -> f64 {
        if self.swap_total == 0 {
            0.0
        } else {
            self.swap_used as f64 / self.swap_total as f64 * 100.0
        }
    }
}

static SNAP: Mutex<MemSnapshot> = Mutex::new(MemSnapshot {
    total: 0,
    used: 0,
    swap_total: 0,
    swap_used: 0,
    available: 0,
    free: 0,
});
static HISTORY: LazyLock<Mutex<History<240>>> = LazyLock::new(|| Mutex::new(History::new()));

pub fn snapshot() -> MemSnapshot {
    *SNAP.lock().unwrap()
}

pub fn sample(sys: &sysinfo::System) {
    let snap = MemSnapshot {
        total: sys.total_memory(),
        used: sys.used_memory(),
        swap_total: sys.total_swap(),
        swap_used: sys.used_swap(),
        available: sys.available_memory(),
        free: sys.free_memory(),
    };
    HISTORY.lock().unwrap().push(snap.pct());
    *SNAP.lock().unwrap() = snap;
}

fn row<'a>(label: &'a str, used: u64, total: u64, pct: f64, width: u16, theme: &Theme) -> Line<'a> {
    let stats = format!(
        "{:>5} / {:<5}",
        meter::fmt_bytes(used),
        meter::fmt_bytes(total)
    );
    let pct_str = format!("{:>3.0}%", pct);
    let fixed = label.chars().count() + 1 + stats.chars().count() + 2 + pct_str.chars().count() + 1;
    let bar_w = (width as usize).saturating_sub(fixed);
    let (on, off) = meter::track(pct / 100.0, bar_w);
    let c = theme.usage(pct);
    Line::from(vec![
        Span::styled(format!("{} ", label), Style::default().fg(theme.dim)),
        Span::styled(format!("{}  ", stats), Style::default().fg(theme.text)),
        Span::styled(on, Style::default().fg(c)),
        Span::styled(off, Style::default().fg(theme.surface)),
        Span::styled(format!(" {}", pct_str), Style::default().fg(c)),
    ])
}

pub fn render(f: &mut Frame, area: Rect, theme: &Theme, is_detailed: bool) {
    if area.height < 2 {
        return;
    }
    let m = snapshot();
    let has_swap = m.swap_total > 0;

    let mut breakdown_rows = 0;
    if is_detailed && area.height >= 4 {
        breakdown_rows = 1;
    }

    let chunks = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(breakdown_rows),
        Constraint::Min(1),
        Constraint::Length(if has_swap { 1 } else { 0 }),
    ])
    .split(area);

    f.render_widget(
        Paragraph::new(row("RAM ", m.used, m.total, m.pct(), area.width, theme)),
        chunks[0],
    );

    if breakdown_rows > 0 {
        let cached = m.total.saturating_sub(m.free).saturating_sub(m.used);
        let wide = area.width >= 44;
        let items = [
            (if wide { "available" } else { "avl" }, m.available),
            (if wide { "free" } else { "fre" }, m.free),
            (if wide { "cache" } else { "cac" }, cached),
        ];
        let spans: Vec<Span> = items
            .iter()
            .enumerate()
            .flat_map(|(i, (k, v))| {
                [
                    Span::styled(
                        format!("{}{} ", if i > 0 { "  " } else { "" }, k),
                        Style::default().fg(theme.dim),
                    ),
                    Span::styled(meter::fmt_bytes(*v), Style::default().fg(theme.text)),
                ]
            })
            .collect();
        f.render_widget(Paragraph::new(Line::from(spans)), chunks[1]);
    }

    let hist = HISTORY.lock().unwrap().recent(1000);
    f.render_widget(
        BlockGraph::new(&hist)
            .max(100.0)
            .colors(theme.secondary, theme.yellow, theme.red),
        chunks[2],
    );

    if has_swap {
        f.render_widget(
            Paragraph::new(row(
                "SWAP",
                m.swap_used,
                m.swap_total,
                m.swap_pct(),
                area.width,
                theme,
            )),
            chunks[3],
        );
    }
}
