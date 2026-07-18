use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;use ratatui::Frame;

use crate::app;

fn usage_color(usage: f64, theme: &app::Theme) -> Color {
    if usage < 50.0 { theme.green }
    else if usage < 80.0 { theme.yellow }
    else { theme.red }
}

pub fn render(f: &mut Frame, area: Rect, theme: &app::Theme) {
    let sys = crate::app::SYS.lock().unwrap();
    // sys.refresh_memory();
    let (total, used, swap_total, swap_used) =
        (sys.total_memory(), sys.used_memory(), sys.total_swap(), sys.used_swap());

    let used_gb = used as f64 / 1_073_741_824.0;
    let total_gb = total as f64 / 1_073_741_824.0;
    let pct = if total > 0 {
        (used as f64 / total as f64) * 100.0
    } else {
        0.0
    };

    // Unused variables removed

    // Compact: RAM gauge + Swap gauge side by side
    let chunks = Layout::vertical([
        Constraint::Length(1), // RAM gauge
        Constraint::Length(1), // Swap gauge
    ])
    .split(area);

    let color = usage_color(pct, theme);
    let stats = format!("{:.1}/{:.1} GiB", used_gb, total_gb);
    let bar_line = crate::widgets::bar::draw_premium_bar(
        "RAM", 4,
        &stats, 13,
        pct / 100.0,
        color, theme.surface,
        area.width,
    );
    f.render_widget(Paragraph::new(bar_line), chunks[0]);

    if swap_total > 0 {
        let swap_used_gb = swap_used as f64 / 1_073_741_824.0;
        let swap_total_gb = swap_total as f64 / 1_073_741_824.0;
        let swap_pct = (swap_used as f64 / swap_total as f64) * 100.0;
        let swap_color = usage_color(swap_pct, theme);
        
        let s_stats = format!("{:.1}/{:.1} GiB", swap_used_gb, swap_total_gb);
        let s_bar_line = crate::widgets::bar::draw_premium_bar(
            "Swap", 4,
            &s_stats, 13,
            swap_pct / 100.0,
            swap_color, theme.surface,
            area.width,
        );
        f.render_widget(Paragraph::new(s_bar_line), chunks[1]);
    } else {
        f.render_widget(
            Paragraph::new(Line::from(Span::styled(" —", Style::default().fg(theme.dim)))),
            chunks[1],
        );
    }
}
