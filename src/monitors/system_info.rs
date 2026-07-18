use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::app;

fn short_cpu(model: &str) -> String {
    let mut s = model
        .replace("(R)", "")
        .replace("(TM)", "")
        .replace(" CPU", "")
        .replace("Intel Core ", "")
        .replace("AMD ", "");
    if let Some(idx) = s.find(" @ ") {
        s.truncate(idx);
    }
    s.trim().to_string()
}

fn short_gpu(model: &str) -> String {
    model
        .replace("NVIDIA GeForce ", "")
        .replace("AMD Radeon ", "")
        .replace("Intel Corporation ", "")
        .replace(" Graphics", "")
        .trim()
        .to_string()
}

pub fn render(f: &mut Frame, area: Rect, theme: &app::Theme) {
    let os = std::fs::read_to_string("/etc/os-release")
        .ok()
        .and_then(|c| {
            for line in c.lines() {
                if let Some(val) = line.strip_prefix("PRETTY_NAME=\"") {
                    return Some(val.trim_end_matches('"').to_string());
                }
            }
            None
        })
        .unwrap_or_else(|| "Linux".to_string());
    let os_short = os.split_whitespace().next().unwrap_or("Linux");

    let hostname = std::fs::read_to_string("/etc/hostname")
        .ok()
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "?".to_string());

    let cpu_model = std::fs::read_to_string("/proc/cpuinfo")
        .ok()
        .and_then(|c| {
            for line in c.lines() {
                if let Some(val) = line.strip_prefix("model name") {
                    if let Some(val) = val.split(':').nth(1) {
                        return Some(val.trim().to_string());
                    }
                }
            }
            None
        })
        .unwrap_or_default();
    let cpu_short = short_cpu(&cpu_model);

    let gpu_full = std::fs::read_to_string("/proc/driver/nvidia/gpus/0000:02:00.0/information")
        .ok()
        .and_then(|s| {
            for line in s.lines() {
                if let Some(val) = line.strip_prefix("Model:") {
                    return Some(val.trim().to_string());
                }
            }
            None
        })
        .unwrap_or_default();
    let gpu = short_gpu(&gpu_full);

    let shell = std::env::var("SHELL")
        .ok()
        .and_then(|s| s.rsplit('/').next().map(|s| s.to_string()))
        .unwrap_or_default();

    let key_style = Style::default().fg(theme.accent);
    let val_style = Style::default().fg(theme.text);

    let mut lines = Vec::new();
    lines.push(Line::from(vec![
        Span::styled(format!("{:>5} ", "OS"), key_style),
        Span::styled(os_short, val_style),
    ]));
    lines.push(Line::from(vec![
        Span::styled(format!("{:>5} ", "Host"), key_style),
        Span::styled(hostname, val_style),
    ]));
    lines.push(Line::from(vec![
        Span::styled(format!("{:>5} ", "CPU"), key_style),
        Span::styled(cpu_short, val_style),
    ]));
    if !gpu.is_empty() {
        lines.push(Line::from(vec![
            Span::styled(format!("{:>5} ", "GPU"), key_style),
            Span::styled(gpu, val_style),
        ]));
    }
    lines.push(Line::from(vec![
        Span::styled(format!("{:>5} ", "Shell"), key_style),
        Span::styled(shell, val_style),
    ]));

    f.render_widget(
        Paragraph::new(lines)
            .style(Style::default().bg(theme.bg))
            .alignment(ratatui::layout::Alignment::Center),
        area,
    );
}
