use std::process::Command;

use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Gauge, Paragraph};
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

pub fn render(f: &mut Frame, area: Rect, theme: &app::Theme, demo: bool) {
    let gpu_data = if demo {
        Some(GpuData {
            util_pct: 30.0,
            temp_c: 62.0,
            mem_used_mb: 1024.0,
            mem_total_mb: 2048.0,
        })
    } else {
        read_gpu()
    };

    if let Some(gpu) = gpu_data {
        let util_color = if gpu.util_pct > 80.0 {
            theme.red
        } else if gpu.util_pct > 40.0 {
            theme.yellow
        } else {
            theme.green
        };
        let temp_color = if gpu.temp_c > 80.0 {
            theme.red
        } else if gpu.temp_c > 60.0 {
            theme.yellow
        } else {
            theme.dim
        };
        let mem_pct = if gpu.mem_total_mb > 0.0 {
            (gpu.mem_used_mb / gpu.mem_total_mb * 100.0) as u16
        } else {
            0
        };
        let mem_color = if mem_pct > 80 { theme.red } else if mem_pct > 50 { theme.yellow } else { theme.green };

        let chunks = Layout::vertical([
            Constraint::Length(1), // text line
            Constraint::Length(2), // util gauge
            Constraint::Length(1), // separator
            Constraint::Length(1), // VRAM label
            Constraint::Length(2), // VRAM gauge
        ])
        .split(area);

        // Info line
        let text = Line::from(vec![
            Span::styled(
                format!(" {}% ", gpu.util_pct as u64),
                Style::default().fg(util_color),
            ),
            Span::styled(
                format!("{}°C ", gpu.temp_c as u64),
                Style::default().fg(temp_color),
            ),
            Span::styled(
                format!("VRAM {:.0}/{:.0} MiB", gpu.mem_used_mb, gpu.mem_total_mb),
                Style::default().fg(theme.dim),
            ),
        ]);
        f.render_widget(
            Paragraph::new(text).style(Style::default().bg(theme.bg)),
            chunks[0],
        );

        // Util gauge
        let gauge = Gauge::default()
            .gauge_style(Style::default().fg(util_color).bg(theme.surface))
            .percent(gpu.util_pct as u16)
            .label(format!("Util {:.0}%", gpu.util_pct));
        f.render_widget(gauge, chunks[1]);

        // Dashed separator
        let sep = "·".repeat(chunks[2].width.saturating_sub(2) as usize);
        f.render_widget(
            Paragraph::new(Line::from(Span::styled(
                format!(" {sep}"),
                Style::default().fg(theme.dim),
            ))),
            chunks[2],
        );

        // VRAM label
        f.render_widget(
            Paragraph::new(Line::from(Span::styled(" VRAM", Style::default().fg(theme.text)))),
            chunks[3],
        );

        // VRAM gauge
        let vram_gauge = Gauge::default()
            .gauge_style(Style::default().fg(mem_color).bg(theme.surface))
            .percent(mem_pct)
            .label(format!(
                "{:.0}%  {:.0}/{:.0} MiB",
                mem_pct, gpu.mem_used_mb, gpu.mem_total_mb
            ));
        f.render_widget(vram_gauge, chunks[4]);
    } else {
        f.render_widget(
            Paragraph::new(Line::from(Span::styled(
                " GPU  N/A (no NVIDIA GPU detected)",
                Style::default().fg(theme.dim),
            )))
            .style(Style::default().bg(theme.bg)),
            area,
        );
    }
}
