use std::time::Instant;

use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::app;

/// Ring buffers for download and upload history
const SHORT_LEN: usize = 60; // ~60 seconds
const LONG_LEN: usize = 240; // ~4 minutes

static mut DL_SHORT: [f64; SHORT_LEN] = [0.0; SHORT_LEN];
static mut DL_LONG: [f64; LONG_LEN] = [0.0; LONG_LEN];
static mut UL_SHORT: [f64; SHORT_LEN] = [0.0; SHORT_LEN];
static mut UL_LONG: [f64; LONG_LEN] = [0.0; LONG_LEN];
static mut SHORT_IDX: usize = 0;
static mut LONG_IDX: usize = 0;

/// Cumulative bytes from previous poll
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

        // Push to short buffer (every tick)
        DL_SHORT[SHORT_IDX] = rx_kbps;
        UL_SHORT[SHORT_IDX] = tx_kbps;
        SHORT_IDX = (SHORT_IDX + 1) % SHORT_LEN;

        // Push to long buffer (every other tick — doubles the span)
        if SHORT_IDX % 2 == 0 {
            let prev_ul = UL_SHORT[(SHORT_IDX + SHORT_LEN - 1) % SHORT_LEN];
            let prev_dl = DL_SHORT[(SHORT_IDX + SHORT_LEN - 1) % SHORT_LEN];
            DL_LONG[LONG_IDX] = (rx_kbps + prev_dl) / 2.0;
            UL_LONG[LONG_IDX] = (tx_kbps + prev_ul) / 2.0;
            LONG_IDX = (LONG_IDX + 1) % LONG_LEN;
        }

        (rx_kbps, tx_kbps)
    }
}

fn sparkline(history: &[f64], idx: usize, width: usize) -> String {
    let n = width.min(history.len());
    if n == 0 {
        return String::new();
    }

    let max_val = history.iter().copied().fold(0.0_f64, |a, b| a.max(b));
    if max_val < 0.01 {
        return "·".repeat(n);
    }

    let chars = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
    let mut line = String::with_capacity(n);

    for i in 0..n {
        let val = history[(idx + i) % history.len()];
        let level = ((val / max_val) * 7.0) as usize;
        line.push(chars[level.min(7)]);
    }

    line
}

fn fmt_kbps(kbps: f64) -> String {
    if kbps > 1024.0 {
        format!("{:.1} MB/s", kbps / 1024.0)
    } else if kbps > 1.0 {
        format!("{:.1} KB/s", kbps)
    } else {
        format!("{:.1} B/s", kbps * 1024.0)
    }
}

pub fn render(f: &mut Frame, area: Rect, theme: &app::Theme, demo: bool) {
    let (rx_kbps, tx_kbps) = if demo {
        (2100.0, 500.0)
    } else {
        read_net_rates()
    };

    // Copy history data locally
    let (dl_short, dl_long, ul_short, _ul_long, short_idx, long_idx) = if demo {
        // Stable pre-built sparkline data: a nice wave pattern
        let mut demo_short = [0.0_f64; SHORT_LEN];
        for i in 0..SHORT_LEN {
            demo_short[i] = 1500.0 + 800.0 * (i as f64 * 0.3).sin().abs();
        }
        let mut demo_long = [0.0_f64; LONG_LEN];
        for i in 0..LONG_LEN {
            demo_long[i] = 1400.0 + 700.0 * (i as f64 * 0.1).sin();
        }
        (demo_short, demo_long, demo_short, demo_long, 0_usize, 0_usize)
    } else {
        unsafe {
            (DL_SHORT, DL_LONG, UL_SHORT, UL_LONG, SHORT_IDX, LONG_IDX)
        }
    };

    let short_width = (area.width.saturating_sub(2) / 2).max(10) as usize;
    let long_width = area.width.saturating_sub(2) as usize;

    let dl_short_spark = sparkline(&dl_short, short_idx, short_width);
    let ul_short_spark = sparkline(&ul_short, short_idx, short_width);
    let dl_long_spark = sparkline(&dl_long, long_idx, long_width);

    // Compact layout
    let chunks = Layout::vertical([
        Constraint::Length(1), // ↓ rate
        Constraint::Length(1), // ↑ rate
        Constraint::Length(1), // short sparkline
        Constraint::Length(1), // long sparkline
    ])
    .split(area);

    // Download rate
    let rx_color = if rx_kbps > 10000.0 {
        theme.red
    } else if rx_kbps > 1000.0 {
        theme.yellow
    } else {
        theme.accent
    };
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::raw(" ↓ "),
            Span::styled(
                format!("{:>8}", fmt_kbps(rx_kbps)),
                Style::default().fg(rx_color),
            ),
        ]))
        .style(Style::default().bg(theme.bg)),
        chunks[0],
    );

    // Upload rate
    let tx_color = if tx_kbps > 5000.0 {
        theme.red
    } else if tx_kbps > 500.0 {
        theme.yellow
    } else {
        theme.secondary
    };
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::raw(" ↑ "),
            Span::styled(
                format!("{:>8}", fmt_kbps(tx_kbps)),
                Style::default().fg(tx_color),
            ),
        ]))
        .style(Style::default().bg(theme.bg)),
        chunks[1],
    );

    // Short-term sparkline: DL and UL side-by-side
    let gap = "  ";
    let short_line = format!(" {}▽{} {}▽{}", &dl_short_spark, gap, &ul_short_spark, gap);
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            &short_line,
            Style::default().fg(theme.green),
        )))
        .style(Style::default().bg(theme.bg)),
        chunks[2],
    );

    // Long-term sparkline
    let long_line = format!(" {}◁{}", &dl_long_spark, gap);
    let avg_dl = if dl_long.iter().sum::<f64>() > 0.0 {
        dl_long.iter().sum::<f64>() / dl_long.len() as f64
    } else {
        0.0
    };
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(&long_line, Style::default().fg(theme.dim)),
            Span::styled(
                format!(" avg {}", fmt_kbps(avg_dl)),
                Style::default().fg(theme.dim),
            ),
        ]))
        .style(Style::default().bg(theme.bg)),
        chunks[3],
    );
}