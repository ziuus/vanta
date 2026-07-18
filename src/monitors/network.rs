use std::time::Instant;

use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::app;

/// Ring buffers for download and upload history
const SHORT_LEN: usize = 60;

static mut DL_SHORT: [f64; SHORT_LEN] = [0.0; SHORT_LEN];
static mut UL_SHORT: [f64; SHORT_LEN] = [0.0; SHORT_LEN];
static mut SHORT_IDX: usize = 0;

static mut PREV_RX: u64 = 0;
static mut PREV_TX: u64 = 0;
static mut PREV_TIME: Option<Instant> = None;
static mut FIRST: bool = true;

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

    unsafe {
        if FIRST {
            PREV_RX = total_rx;
            PREV_TX = total_tx;
            PREV_TIME = Some(Instant::now());
            FIRST = false;
            return (0.0, 0.0);
        }

        let now = Instant::now();
        let elapsed = PREV_TIME.map(|t| t.elapsed().as_secs_f64()).unwrap_or(1.0);
        let drx = total_rx.saturating_sub(PREV_RX);
        let dtx = total_tx.saturating_sub(PREV_TX);

        PREV_RX = total_rx;
        PREV_TX = total_tx;
        PREV_TIME = Some(now);

        let rx_kbps = drx as f64 / 1024.0 / elapsed.max(0.1);
        let tx_kbps = dtx as f64 / 1024.0 / elapsed.max(0.1);

        DL_SHORT[SHORT_IDX] = rx_kbps;
        UL_SHORT[SHORT_IDX] = tx_kbps;
        SHORT_IDX = (SHORT_IDX + 1) % SHORT_LEN;

        (rx_kbps, tx_kbps)
    }
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
    let (dl_short, ul_short, _short_idx) = unsafe { (DL_SHORT, UL_SHORT, SHORT_IDX) };

    let _gauge_width = area.width.saturating_sub(2) as usize;
    let dl_max = max_rate(&dl_short).max(1.0);
    let ul_max = max_rate(&ul_short).max(1.0);
    let dl_pct = ((rx_kbps / dl_max) * 100.0) as u16;
    let ul_pct = ((tx_kbps / ul_max) * 100.0) as u16;

    let chunks = Layout::vertical([
        Constraint::Length(1), // dl gauge
        Constraint::Length(1), // ul gauge
    ])
    .split(area);

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

    let dl_stats = format!("{}", fmt_kbps(rx_kbps));
    let dl_line = crate::widgets::bar::draw_premium_bar(
        "↓ RX", 6,
        &dl_stats, 13,
        (dl_pct as f64) / 100.0,
        rx_color,
        theme.surface,
        chunks[0].width,
    );
    f.render_widget(Paragraph::new(dl_line), chunks[0]);

    // Upload
    let ul_stats = format!("{}", fmt_kbps(tx_kbps));
    let ul_line = crate::widgets::bar::draw_premium_bar(
        "↑ TX", 6,
        &ul_stats, 13,
        (ul_pct as f64) / 100.0,
        tx_color,
        theme.surface,
        chunks[1].width,
    );
    f.render_widget(Paragraph::new(ul_line), chunks[1]);
}
