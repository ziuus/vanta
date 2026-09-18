use std::sync::{LazyLock, Mutex};
use std::time::Instant;

use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::monitors::history::History;
use crate::theme::Theme;
use crate::widgets::meter;
use crate::widgets::block_graph::BlockGraph;

const HIST: usize = 240;

struct NetState {
    rx: History<HIST>,
    tx: History<HIST>,
    /// Current rates in KiB/s.
    rx_kbps: f64,
    tx_kbps: f64,
    /// Totals since boot, bytes.
    rx_total: u64,
    tx_total: u64,
    prev: Option<(u64, u64, Instant)>,
}

static NET: LazyLock<Mutex<NetState>> = LazyLock::new(|| {
    Mutex::new(NetState {
        rx: History::new(),
        tx: History::new(),
        rx_kbps: 0.0,
        tx_kbps: 0.0,
        rx_total: 0,
        tx_total: 0,
        prev: None,
    })
});

#[derive(Clone, Copy)]
pub struct NetSnapshot {
    pub rx_kbps: f64,
    pub tx_kbps: f64,
    pub rx_total: u64,
    pub tx_total: u64,
}

pub fn snapshot() -> NetSnapshot {
    let n = NET.lock().unwrap();
    NetSnapshot {
        rx_kbps: n.rx_kbps,
        tx_kbps: n.tx_kbps,
        rx_total: n.rx_total,
        tx_total: n.tx_total,
    }
}

fn read_counters() -> Option<(u64, u64)> {
    let s = std::fs::read_to_string("/proc/net/dev").ok()?;
    let (mut rx, mut tx) = (0u64, 0u64);
    for line in s.lines().skip(2) {
        let Some((iface, rest)) = line.split_once(':') else {
            continue;
        };
        if iface.trim() == "lo" {
            continue;
        }
        let f: Vec<&str> = rest.split_whitespace().collect();
        if f.len() >= 9 {
            rx += f[0].parse::<u64>().unwrap_or(0);
            tx += f[8].parse::<u64>().unwrap_or(0);
        }
    }
    Some((rx, tx))
}

pub fn sample() {
    let Some((rx, tx)) = read_counters() else {
        return;
    };
    let now = Instant::now();
    let mut n = NET.lock().unwrap();
    if let Some((prx, ptx, pt)) = n.prev {
        let dt = now.duration_since(pt).as_secs_f64().max(0.05);
        n.rx_kbps = rx.saturating_sub(prx) as f64 / 1024.0 / dt;
        n.tx_kbps = tx.saturating_sub(ptx) as f64 / 1024.0 / dt;
        let (r, t) = (n.rx_kbps, n.tx_kbps);
        n.rx.push(r);
        n.tx.push(t);
    }
    n.rx_total = rx;
    n.tx_total = tx;
    n.prev = Some((rx, tx, now));
}

/// Two stacked graphs (down / up), each auto-scaled to its own recent peak.
pub fn render(f: &mut Frame, area: Rect, theme: &Theme) {
    if area.height < 3 {
        return;
    }
    let (rx_h, tx_h, snap) = {
        let n = NET.lock().unwrap();
        (
            n.rx.recent(1000),
            n.tx.recent(1000),
            NetSnapshot {
                rx_kbps: n.rx_kbps,
                tx_kbps: n.tx_kbps,
                rx_total: n.rx_total,
                tx_total: n.tx_total,
            },
        )
    };

    let split = Layout::vertical([Constraint::Length(1), Constraint::Length(1), Constraint::Min(0)]).split(area);
    let peak = rx_h.iter().chain(tx_h.iter()).copied().fold(0.0, f64::max).max(1.0);

    let mk_line = |arrow: &str, rate: f64, peak: f64, color, total| {
        let mut head = Line::from(vec![
            Span::styled(format!("{} ", arrow), Style::default().fg(color)),
            Span::styled(
                format!("{:>10}", meter::fmt_kbps(rate)),
                Style::default().fg(color),
            ),
            Span::styled(
                format!("  peak {}", meter::fmt_kbps(peak)),
                Style::default().fg(theme.dim),
            ),
        ]);
        if area.width >= 44 {
            head.spans.push(Span::styled(
                format!("  total {}", meter::fmt_bytes(total)),
                Style::default().fg(theme.dim),
            ));
        }
        head
    };

    f.render_widget(Paragraph::new(mk_line("↓", snap.rx_kbps, peak, theme.accent, snap.rx_total)), split[0]);
    f.render_widget(Paragraph::new(mk_line("↑", snap.tx_kbps, peak, theme.secondary, snap.tx_total)), split[1]);

    if split[2].height > 0 {
        let rx_cur = rx_h.last().copied().unwrap_or(0.0);
        let rx_dyn = if peak > 0.0 { theme.usage(rx_cur / peak * 100.0) } else { theme.accent };
        let tx_cur = tx_h.last().copied().unwrap_or(0.0);
        let tx_dyn = if peak > 0.0 { theme.usage(tx_cur / peak * 100.0) } else { theme.secondary };
        
        f.render_widget(
            BlockGraph::new(&rx_h)
                .data2(&tx_h)
                .max(peak)
                .colors(rx_dyn, theme.yellow, theme.red)
                .colors2(tx_dyn, theme.yellow, theme.red),
            split[2],
        );
    }
}
