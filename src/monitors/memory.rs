use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Gauge, Paragraph};
use ratatui::Frame;

use crate::app;

fn usage_color(usage: f64, theme: &app::Theme) -> Color {
    if usage < 50.0 { theme.green }
    else if usage < 80.0 { theme.yellow }
    else { theme.red }
}

/// Read specific fields from /proc/meminfo, returns (cached_kb, buffers_kb, slab_kb, sreclaimable_kb)
fn read_meminfo() -> (u64, u64, u64, u64) {
    let content = std::fs::read_to_string("/proc/meminfo").unwrap_or_default();
    let mut cached = 0u64;
    let mut buffers = 0u64;
    let mut slab = 0u64;
    let mut sreclaimable = 0u64;
    for line in content.lines() {
        if let Some(val) = line.strip_prefix("Cached:") {
            cached = val.trim().split_whitespace().next().and_then(|s| s.parse().ok()).unwrap_or(0);
        } else if let Some(val) = line.strip_prefix("Buffers:") {
            buffers = val.trim().split_whitespace().next().and_then(|s| s.parse().ok()).unwrap_or(0);
        } else if let Some(val) = line.strip_prefix("Slab:") {
            slab = val.trim().split_whitespace().next().and_then(|s| s.parse().ok()).unwrap_or(0);
        } else if let Some(val) = line.strip_prefix("SReclaimable:") {
            sreclaimable = val.trim().split_whitespace().next().and_then(|s| s.parse().ok()).unwrap_or(0);
        }
    }
    (cached, buffers, slab, sreclaimable)
}

fn fmt_gb(kb: u64) -> String {
    let gb = kb as f64 / 1_048_576.0;
    format!("{:.1}", gb)
}

pub fn render(f: &mut Frame, area: Rect, theme: &app::Theme, demo: bool) {
    // ── Collect data (real or demo) ──
    let (total, used, available, swap_total, swap_used, cached_kb, buffers_kb, slab_kb, sreclaimable_kb) =
        if demo {
            // Stable data for screenshots: RAM 55% (8.2/14.9 GiB), Swap 12% (1.1/9.3 GiB)
            let total_mem: u64 = (14.9_f64 * 1_073_741_824.0) as u64;    // 14.9 GiB
            let used_mem: u64 = (8.2_f64 * 1_073_741_824.0) as u64;      // 8.2 GiB
            let avail_mem: u64 = total_mem - used_mem;                    // ~6.7 GiB
            let swap_total: u64 = (9.3_f64 * 1_073_741_824.0) as u64;    // 9.3 GiB
            let swap_used: u64 = (swap_total as f64 * 0.12) as u64;      // 12% → ~1.1 GiB
            let cached_kb: u64 = (2.1_f64 * 1_048_576.0) as u64;         // 2.1 GiB
            let buffers_kb: u64 = (0.3_f64 * 1_048_576.0) as u64;        // 0.3 GiB
            let slab_kb: u64 = (0.8_f64 * 1_048_576.0) as u64;           // 0.8 GiB
            let sreclaimable_kb: u64 = (1.2_f64 * 1_048_576.0) as u64;   // 1.2 GiB
            (total_mem, used_mem, avail_mem, swap_total, swap_used, cached_kb, buffers_kb, slab_kb, sreclaimable_kb)
        } else {
            let mut system = sysinfo::System::new_all();
            system.refresh_memory();

            let total = system.total_memory();
            let used = system.used_memory();
            let available = system.available_memory();

            let swap_total = system.total_swap();
            let swap_used = system.used_swap();

            let (cached_kb, buffers_kb, slab_kb, sreclaimable_kb) = read_meminfo();
            (total, used, available, swap_total, swap_used, cached_kb, buffers_kb, slab_kb, sreclaimable_kb)
        };

    let used_gb = used as f64 / 1_073_741_824.0;
    let total_gb = total as f64 / 1_073_741_824.0;
    let pct = if total > 0 {
        (used as f64 / total as f64) * 100.0
    } else {
        0.0
    };

    let swap_used_gb = swap_used as f64 / 1_073_741_824.0;
    let swap_total_gb = swap_total as f64 / 1_073_741_824.0;
    let swap_pct = if swap_total > 0 { (swap_used as f64 / swap_total as f64) * 100.0 } else { 0.0 };

    let cached_gb = fmt_gb(cached_kb);
    let buffers_gb = fmt_gb(buffers_kb);
    let sreclaimable_gb = fmt_gb(sreclaimable_kb);

    // Layout: RAM label | RAM gauge | breakdown line | separator | Swap label | Swap gauge
    let chunks = Layout::vertical([
        Constraint::Length(1),                          // RAM label
        Constraint::Length(2),                          // RAM gauge
        Constraint::Length(1),                          // breakdown (avail + cached/buf/slab)
        Constraint::Length(1),                          // separator
        Constraint::Length(1),                          // Swap label
        Constraint::Length(2),                          // Swap gauge
    ])
    .split(area);

    // ── RAM label ──
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(" RAM", Style::default().fg(theme.text)))),
        chunks[0],
    );

    // ── RAM gauge ──
    let color = usage_color(pct, theme);
    let gauge = Gauge::default()
        .gauge_style(Style::default().fg(color).bg(theme.surface))
        .percent(pct as u16)
        .label(format!("{:.1}%  {:.1} GiB / {:.1} GiB", pct, used_gb, total_gb));
    f.render_widget(gauge, chunks[1]);

    // ── Breakdown line ──
    let mut parts = vec![];
    parts.push(format!("avail {}", fmt_gb(available as u64 / 1024)));
    if cached_kb > 0 {
        parts.push(format!("cached {}G", cached_gb));
    }
    if buffers_kb > 0 {
        parts.push(format!("buf {}G", buffers_gb));
    }
    if sreclaimable_kb > 0 {
        parts.push(format!("sRecl {}G", sreclaimable_gb));
    }
    if slab_kb > 0 {
        parts.push(format!("slab {}", fmt_gb(slab_kb)));
    }
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            format!("  {}", parts.join("  ")),
            Style::default().fg(theme.dim),
        ))),
        chunks[2],
    );

    // ── Dashed separator ──
    let sep = "·".repeat(chunks[3].width.saturating_sub(2) as usize);
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(format!(" {}", sep), Style::default().fg(theme.dim)))),
        chunks[3],
    );

    // ── Swap label ──
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(" Swap", Style::default().fg(theme.text)))),
        chunks[4],
    );

    // ── Swap gauge ──
    if swap_total > 0 {
        let swap_color = usage_color(swap_pct, theme);
        let swap_gauge = Gauge::default()
            .gauge_style(Style::default().fg(swap_color).bg(theme.surface))
            .percent(swap_pct as u16)
            .label(format!("{:.1}%  {:.1} GiB / {:.1} GiB", swap_pct, swap_used_gb, swap_total_gb));
        f.render_widget(swap_gauge, chunks[5]);
    } else {
        f.render_widget(
            Paragraph::new(Span::styled(" —", Style::default().fg(theme.dim))),
            chunks[5],
        );
    }
}
