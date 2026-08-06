use std::sync::{LazyLock, Mutex};
use std::time::Instant;

use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::app;

/// Ring buffers for download and upload history
const SHORT_LEN: usize = 60;

struct NetState {
    dl_short: [f64; SHORT_LEN],
    ul_short: [f64; SHORT_LEN],
    short_idx: usize,
    prev_rx: u64,
    prev_tx: u64,
    prev_time: Option<Instant>,
    first: bool,
}

static NET: LazyLock<Mutex<NetState>> = LazyLock::new(|| {
    Mutex::new(NetState {
        dl_short: [0.0; SHORT_LEN],
        ul_short: [0.0; SHORT_LEN],
        short_idx: 0,
        prev_rx: 0,
        prev_tx: 0,
        prev_time: None,
        first: true,
    })
});

fn read_net_rates() -> (f64, f64) {
    let counters = std::fs::read_to_string("/proc/net/dev").ok();
    let counters = match counters {
        Some(c) => c,
        None => return (0.0, 0.0),
    };

    let mut total_rx: u64 = 0;
    let mut total_tx: u64 = 0;
    for line in counters.lines().skip(2) {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 10 {
            let rx: u64 = parts[1].parse().unwrap_or(0);
            let tx: u64 = parts[9].parse().unwrap_or(0);
            total_rx += rx;
            total_tx += tx;
        }
    }

    let mut net = NET.lock().unwrap();
    if net.first {
        net.prev_rx = total_rx;
        net.prev_tx = total_tx;
        net.prev_time = Some(Instant::now());
        net.first = false;
        return (0.0, 0.0);
    }

    let now = Instant::now();
    let elapsed = net.prev_time.map(|t| t.elapsed().as_secs_f64()).unwrap_or(1.0);
    let drx = total_rx.saturating_sub(net.prev_rx);
    let dtx = total_tx.saturating_sub(net.prev_tx);

    net.prev_rx = total_rx;
    net.prev_tx = total_tx;
    net.prev_time = Some(now);

    let rx_kbps = drx as f64 / 1024.0 / elapsed.max(0.1);
    let tx_kbps = dtx as f64 / 1024.0 / elapsed.max(0.1);

    let idx = net.short_idx;
    net.dl_short[idx] = rx_kbps;
    net.ul_short[idx] = tx_kbps;
    net.short_idx = (idx + 1) % SHORT_LEN;

    (rx_kbps, tx_kbps)
}

fn max_rate(history: &[f64]) -> f64 {
    history.iter().copied().fold(0.0_f64, |a, b| a.max(b))
}

fn fmt_kbps(kbps: f64) -> String {
    if kbps > 1024.0 {
        format!("{:>5.1} MB/s", kbps / 1024.0)
    } else if kbps > 1.0 {
        format!("{:>5.1} KB/s", kbps)
    } else {
        format!("{:>5.1}  B/s", kbps * 1024.0)
    }
}

pub fn render(f: &mut Frame, area: Rect, theme: &app::Theme) {
    let (rx_kbps, tx_kbps) = read_net_rates();

    // Copy history locally
    let (dl_short, ul_short, idx) = {
        let net = NET.lock().unwrap();
        (net.dl_short, net.ul_short, net.short_idx)
    };

    let rx_color = if rx_kbps > 10000.0 {
        theme.red
    } else if rx_kbps > 1000.0 {
        theme.yellow
    } else {
        theme.accent
    };
    let tx_color = if tx_kbps > 5000.0 {
        theme.red
    } else if tx_kbps > 500.0 {
        theme.yellow
    } else {
        theme.secondary
    };

    // Split the panel in half: download on top, upload below. Each half is a
    // header line plus a braille history graph that fills the leftover rows.
    let halves = Layout::vertical([Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)]).split(area);

    for (half, label, rate, hist, color, peak_label) in [
        (halves[0], "\u{2193} RX", rx_kbps, &dl_short, rx_color, "peak"),
        (halves[1], "\u{2191} TX", tx_kbps, &ul_short, tx_color, "peak"),
    ] {
        let rows = Layout::vertical([Constraint::Length(1), Constraint::Min(1)]).split(half);

        let peak = max_rate(hist).max(1.0);
        let header = Line::from(vec![
            Span::styled(format!("{} ", label), Style::default().fg(theme.dim)),
            Span::styled(
                format!("{:>10}", fmt_kbps(rate)),
                Style::default()
                    .fg(color)
                    .add_modifier(ratatui::style::Modifier::BOLD),
            ),
            Span::styled(
                format!("  {} {}", peak_label, fmt_kbps(peak)),
                Style::default().fg(theme.dim),
            ),
        ]);
        f.render_widget(Paragraph::new(header), rows[0]);

        if rows[1].height == 0 {
            continue;
        }

        // Oldest-to-newest window sized to the graph (2 samples per cell).
        let want = (rows[1].width as usize * 2).min(SHORT_LEN);
        let series: Vec<f64> = (0..want)
            .map(|i| hist[(idx + SHORT_LEN - 1 - i) % SHORT_LEN])
            .rev()
            .collect();

        // Network has no natural ceiling — scale to the window peak so the
        // shape is always visible regardless of absolute throughput.
        let graph = crate::widgets::block_graph::BlockGraph::new(&series)
            .min(0.0)
            .max(peak)
            .colors(color, theme.yellow, theme.red);
        f.render_widget(graph, rows[1]);
    }
}
