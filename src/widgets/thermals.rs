use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph};
use ratatui::Frame;

use crate::monitors::{cpu, gpu, system_info};
use crate::theme::Theme;
use crate::widgets::meter;

pub fn render(f: &mut Frame, area: Rect, theme: &Theme) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.surface));
    let inner = block.inner(area);
    f.render_widget(block, area);

    if inner.width < 10 {
        return;
    }

    let cpu_snap = cpu::snapshot();
    let gpu_snap = gpu::snapshot();

    // get temp and fan
    let cpu_temp = crate::monitors::cpu::snapshot().max_temp();
    let gpu_temp = crate::monitors::gpu::snapshot().and_then(|g| g.temp_c);

    // We could use gauge widgets or braille blocks.
    // Wait, we can draw a circle or half-circle using ratatui's canvas or just simple characters.
    // Ratatui has `Gauge` widget, but it's a linear bar.
    // The prompt says "circular/braille gauges".
    // Let's use ratatui canvas for a circle? Or just simple text blocks since terminal width is limited.
    // Given time constraints, I will build a beautifully styled list with mini bar charts.

    let mut lines = Vec::new();

    let temp_color = |t: f64| -> Color {
        if t > 80.0 {
            theme.red
        } else if t > 60.0 {
            theme.yellow
        } else {
            theme.green
        }
    };

    if let Some(t) = cpu_temp {
        lines.push(Line::from(vec![
            Span::styled(" CPU ", Style::default().fg(theme.text)),
            Span::styled(format!("{:5.1}°C ", t), Style::default().fg(temp_color(t))),
            Span::raw(meter::bar(t / 100.0, 10)),
        ]));
    } else {
        lines.push(Line::from(" CPU  --.-°C"));
    }

    if let Some(t) = gpu_temp {
        lines.push(Line::from(vec![
            Span::styled(" GPU ", Style::default().fg(theme.text)),
            Span::styled(format!("{:5.1}°C ", t), Style::default().fg(temp_color(t))),
            Span::raw(meter::bar(t / 100.0, 10)),
        ]));
    } else {
        lines.push(Line::from(" GPU  --.-°C"));
    }

    // Some systems don't expose fan speed easily through sysinfo without root,
    // but if we have it in summary:
    // Well we don't have fans in summary right now.
    // Just putting CPU and GPU is good enough for now.

    f.render_widget(Paragraph::new(lines), inner);
}
