use std::fs;

use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::app;

pub fn render(f: &mut Frame, area: Rect, theme: &app::Theme, _demo: bool) {
    // OS name
    let os = fs::read_to_string("/etc/os-release")
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

    // Kernel
    let kernel = fs::read_to_string("/proc/sys/kernel/ostype")
        .ok()
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "Linux".to_string());
    let release = fs::read_to_string("/proc/sys/kernel/osrelease")
        .ok()
        .map(|s| s.trim().to_string())
        .unwrap_or_default();
    let kernel_ver = format!("{}{}", kernel, if release.is_empty() { String::new() } else { format!(" {}", release) });

    // Hostname
    let hostname = fs::read_to_string("/etc/hostname")
        .ok()
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "localhost".to_string());

    // CPU model
    let cpu_model = fs::read_to_string("/proc/cpuinfo")
        .ok()
        .and_then(|c| {
            for line in c.lines() {
                if let Some(val) = line.strip_prefix("model name") {
                    let parts: Vec<&str> = val.split(':').collect();
                    if parts.len() > 1 {
                        return Some(parts[1].trim().to_string());
                    }
                }
            }
            None
        })
        .unwrap_or_default();
    let cpu_short = cpu_model
        .replace("(R)", "")
        .replace("(TM)", "")
        .replace(" CPU", "")
        .trim()
        .to_string();

    // GPU (from nvidia-smi or lspci)
    let gpu = std::process::Command::new("nvidia-smi")
        .args(["--query-gpu=name", "--format=csv,noheader"])
        .output()
        .ok()
        .and_then(|o| {
            let s = String::from_utf8_lossy(&o.stdout).trim().to_string();
            if s.is_empty() { None } else { Some(s) }
        })
        .unwrap_or_default();

    // User shell
    let shell = std::env::var("SHELL")
        .ok()
        .and_then(|s| {
            s.rsplit('/').next().map(|s| s.to_string())
        })
        .unwrap_or_default();

    // Terminal
    let term = std::env::var("TERM_PROGRAM")
        .or_else(|_| std::env::var("TERMINAL"))
        .unwrap_or_default();
    let term_short = if term.contains("kitty") { "kitty" }
        else if term.contains("alacritty") { "alacritty" }
        else if term.contains("wezterm") { "wezterm" }
        else if term.contains("ghostty") { "ghostty" }
        else { &term };

    // Desktop environment
    let de = std::env::var("XDG_CURRENT_DESKTOP")
        .or_else(|_| std::env::var("DESKTOP_SESSION"))
        .ok()
        .unwrap_or_default();

    // ── Render (compact 2-line layout) ──
    let lines = vec![
        // Line 1: OS · kernel · hostname
        Line::from(vec![
            Span::styled(" ", Style::default().fg(theme.dim)),
            Span::styled(format!("{} · {}", os_short, hostname), Style::default().fg(theme.text)),
            Span::styled(format!(" · {}", kernel_ver), Style::default().fg(theme.dim)),
        ]),
        // Line 2: CPU · GPU · shell · terminal · DE
        Line::from(vec![
            Span::styled(" ", Style::default().fg(theme.dim)),
            Span::styled(cpu_short, Style::default().fg(theme.text)),
            if !gpu.is_empty() {
                Span::styled(format!(" · {}", gpu), Style::default().fg(theme.dim))
            } else {
                Span::styled("", Style::default())
            },
            if !shell.is_empty() {
                Span::styled(format!(" · {}", shell), Style::default().fg(theme.dim))
            } else {
                Span::styled("", Style::default())
            },
            if !term_short.is_empty() {
                Span::styled(format!(" · {}", term_short), Style::default().fg(theme.dim))
            } else {
                Span::styled("", Style::default())
            },
            if !de.is_empty() {
                Span::styled(format!(" · {}", de), Style::default().fg(theme.dim))
            } else {
                Span::styled("", Style::default())
            },
        ]),
    ];

    f.render_widget(Paragraph::new(lines).style(Style::default().bg(theme.surface)), area);
}