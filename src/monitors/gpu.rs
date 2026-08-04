use std::process::Command;

use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::Style;
use ratatui::symbols::line;
use ratatui::text::{Line, Span};
use ratatui::widgets::{LineGauge, Paragraph};
use ratatui::Frame;

use crate::app;

struct GpuData {
    util_pct: f64,
    temp_c: f64,
    mem_used_mb: f64,
    mem_total_mb: f64,
}

fn read_gpu() -> Option<GpuData> {
    let out = Command::new("nvidia-smi")
        .args([
            "--query-gpu=utilization.gpu,temperature.gpu,memory.used,memory.total",
            "--format=csv,noheader,nounits",
        ])
        .output()
        .ok()?;
    let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
    let parts: Vec<f64> = s.split(',').filter_map(|x| x.trim().parse().ok()).collect();
    if parts.len() == 4 {
        Some(GpuData {
            util_pct: parts[0],
            temp_c: parts[1],
            mem_used_mb: parts[2],
            mem_total_mb: parts[3],
        })
    } else {
        None
    }
}

pub fn render(f: &mut Frame, area: Rect, theme: &app::Theme) {
    let gpu_data = read_gpu();

    if let Some(gpu) = gpu_data {
        let util_color = if gpu.util_pct > 95.0 {
            theme.red
        } else if gpu.util_pct > 80.0 {
            theme.yellow
        } else {
            theme.accent
        };
        let _mem_pct = if gpu.mem_total_mb > 0.0 {
            (gpu.mem_used_mb / gpu.mem_total_mb * 100.0) as u16
        } else {
            0
        };

        // Single info line + gauge, btop-style
        let chunks = Layout::vertical([
            Constraint::Length(1), // text line
            Constraint::Length(1), // gauge
        ])
        .split(area);

        let text = Line::from(vec![
            Span::styled(
                format!(" {}% ", gpu.util_pct as u64),
                Style::default().fg(util_color),
            ),
            Span::styled(
                format!("{}°C ", gpu.temp_c as u64),
                Style::default().fg(theme.dim),
            ),
            Span::styled(
                format!("VRAM {:.0}/{:.0} MiB", gpu.mem_used_mb, gpu.mem_total_mb),
                Style::default().fg(theme.text),
            ),
        ]);
        f.render_widget(Paragraph::new(text), chunks[0]);

        let gauge = LineGauge::default()
            .filled_style(Style::default().fg(util_color).bg(theme.surface))
            .ratio((gpu.util_pct / 100.0).clamp(0.0, 1.0))
            .label(format!("Util {:.0}%", gpu.util_pct))
            .line_set(line::THICK);
        f.render_widget(gauge, chunks[1]);
    } else {
        f.render_widget(
            Paragraph::new(Line::from(Span::styled(
                " N/A (no NVIDIA GPU detected)",
                Style::default().fg(theme.dim),
            )))
            .style(Style::default().bg(theme.bg)),
            area,
        );
    }
}
