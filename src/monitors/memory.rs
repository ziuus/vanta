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

pub fn render(f: &mut Frame, area: Rect, theme: &app::Theme, demo: bool) {
    // ── Collect data (real or demo) ──
    let (total, used, swap_total, swap_used) =
        if demo {
            // Stable data for screenshots: RAM 55% (8.2/14.9 GiB), Swap 12% (1.1/9.3 GiB)
            let total_mem: u64 = (14.9_f64 * 1_073_741_824.0) as u64;    // 14.9 GiB
            let used_mem: u64 = (8.2_f64 * 1_073_741_824.0) as u64;      // 8.2 GiB
            let swap_total: u64 = (9.3_f64 * 1_073_741_824.0) as u64;    // 9.3 GiB
            let swap_used: u64 = (swap_total as f64 * 0.12) as u64;      // 12% → ~1.1 GiB
            (total_mem, used_mem, swap_total, swap_used)
        } else {
            let mut system = sysinfo::System::new_all();
            system.refresh_memory();
            (system.total_memory(), system.used_memory(), system.total_swap(), system.used_swap())
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

    // Layout: RAM (label + gauge) | Swap (label + gauge)
    let chunks = Layout::vertical([
        Constraint::Length(1),  // RAM label
        Constraint::Length(1),  // RAM gauge (single line, label shows percent + used/total)
        Constraint::Length(1),  // Swap label
        Constraint::Length(1),  // Swap gauge
    ])
    .split(area);

    // ── RAM ──
    let color = usage_color(pct, theme);
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            format!(" RAM {:.0}%  {:.1} GiB / {:.1} GiB", pct, used_gb, total_gb),
            Style::default().fg(theme.text),
        ))),
        chunks[0],
    );
    let gauge = Gauge::default()
        .gauge_style(Style::default().fg(color).bg(theme.surface))
        .percent(pct as u16)
        .label("");
    f.render_widget(gauge, chunks[1]);

    // ── Swap ──
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            format!(" Swap {:.0}%  {:.1} GiB / {:.1} GiB", swap_pct, swap_used_gb, swap_total_gb),
            Style::default().fg(theme.text),
        ))),
        chunks[2],
    );
    if swap_total > 0 {
        let swap_color = usage_color(swap_pct, theme);
        let swap_gauge = Gauge::default()
            .gauge_style(Style::default().fg(swap_color).bg(theme.surface))
            .percent(swap_pct as u16)
            .label("");
        f.render_widget(swap_gauge, chunks[3]);
    } else {
        f.render_widget(
            Paragraph::new(Span::styled(" —", Style::default().fg(theme.dim))),
            chunks[3],
        );
    }
}
