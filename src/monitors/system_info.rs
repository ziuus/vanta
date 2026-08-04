use ratatui::layout::Rect;
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

    let gpu_full = std::fs::read_dir("/proc/driver/nvidia/gpus")
        .ok()
        .and_then(|entries| {
            entries.flatten().find_map(|entry| {
                let info = std::fs::read_to_string(entry.path().join("information")).ok()?;
                info.lines()
                    .find_map(|line| line.strip_prefix("Model:").map(|v| v.trim().to_string()))
            })
        })
        .unwrap_or_default();
    let gpu = short_gpu(&gpu_full);

    let shell = std::env::var("SHELL")
        .ok()
        .and_then(|s| s.rsplit('/').next().map(|s| s.to_string()))
        .unwrap_or_default();

    let uptime = std::fs::read_to_string("/proc/uptime")
        .ok()
        .and_then(|c| {
            let secs: f64 = c.split_whitespace().next()?.parse().ok()?;
            let days = (secs / 86400.0) as u64;
            let hours = ((secs % 86400.0) / 3600.0) as u64;
            let mins = ((secs % 3600.0) / 60.0) as u64;
            let mut s = String::new();
            if days > 0 {
                s.push_str(&format!("{}d ", days));
            }
            if hours > 0 {
                s.push_str(&format!("{}h ", hours));
            }
            s.push_str(&format!("{}m", mins));
            Some(s)
        })
        .unwrap_or_else(|| "?".to_string());

    let battery = std::fs::read_to_string("/sys/class/power_supply/BAT0/capacity")
        .ok()
        .map(|s| format!("{}%", s.trim()))
        .unwrap_or_else(|| "AC".to_string());

    let key_style = Style::default().fg(theme.dim);
    let val_style = Style::default()
        .fg(theme.text)
        .add_modifier(ratatui::style::Modifier::BOLD);

    let mut left_lines = Vec::new();
    left_lines.push(Line::from(vec![
        Span::styled(format!("{:>7} ", "OS"), key_style),
        Span::styled(os_short, val_style),
    ]));
    left_lines.push(Line::from(vec![
        Span::styled(format!("{:>7} ", "Host"), key_style),
        Span::styled(hostname, val_style),
    ]));
    left_lines.push(Line::from(vec![
        Span::styled(format!("{:>7} ", "CPU"), key_style),
        Span::styled(cpu_short, val_style),
    ]));
    if !gpu.is_empty() {
        left_lines.push(Line::from(vec![
            Span::styled(format!("{:>7} ", "GPU"), key_style),
            Span::styled(gpu, val_style),
        ]));
    }

    let mut right_lines = Vec::new();
    right_lines.push(Line::from(vec![
        Span::styled(format!("{:>7} ", "Uptime"), key_style),
        Span::styled(uptime, val_style),
    ]));
    right_lines.push(Line::from(vec![
        Span::styled(format!("{:>7} ", "Battery"), key_style),
        Span::styled(battery, val_style),
    ]));
    right_lines.push(Line::from(vec![
        Span::styled(format!("{:>7} ", "Shell"), key_style),
        Span::styled(shell, val_style),
    ]));

    let chunks = ratatui::layout::Layout::horizontal([
        ratatui::layout::Constraint::Ratio(1, 2),
        ratatui::layout::Constraint::Ratio(1, 2),
    ])
    .split(area);

    f.render_widget(
        Paragraph::new(left_lines).alignment(ratatui::layout::Alignment::Right),
        chunks[0],
    );
    f.render_widget(
        Paragraph::new(right_lines).alignment(ratatui::layout::Alignment::Left),
        chunks[1],
    );
}

/// Collected data for the Overview neofetch hero.
pub(crate) struct NeoData {
    pub os: String,
    pub host: String,
    pub kernel: String,
    pub uptime: String,
    pub shell: String,
    pub resolution: String,
    pub cpu: String,
    pub gpu: String,
    pub memory: String,
    pub bat: String,
}

fn read_os_pretty() -> String {
    std::fs::read_to_string("/etc/os-release")
        .ok()
        .and_then(|c| {
            for line in c.lines() {
                if let Some(val) = line.strip_prefix("PRETTY_NAME=\"") {
                    return Some(val.trim_end_matches('"').to_string());
                }
            }
            None
        })
        .unwrap_or_else(|| "Linux".to_string())
}

fn read_cpu_short() -> String {
    let model = std::fs::read_to_string("/proc/cpuinfo")
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
    short_cpu(&model)
}

fn read_gpu_short() -> String {
    let gpu_full = std::fs::read_dir("/proc/driver/nvidia/gpus")
        .ok()
        .and_then(|entries| {
            entries.flatten().find_map(|entry| {
                let info = std::fs::read_to_string(entry.path().join("information")).ok()?;
                info.lines()
                    .find_map(|line| line.strip_prefix("Model:").map(|v| v.trim().to_string()))
            })
        })
        .unwrap_or_default();
    short_gpu(&gpu_full)
}

fn fmt_ram(bytes: u64) -> String {
    if bytes >= 1024 * 1024 * 1024 {
        format!("{:.1}G", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    } else if bytes >= 1024 * 1024 {
        format!("{:.0}M", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{}K", bytes / 1024)
    }
}

pub(crate) fn collect_neofetch(sum: &app::Summary, w: usize, h: usize) -> NeoData {
    let host = std::fs::read_to_string("/etc/hostname")
        .ok()
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "?".to_string());

    let kernel = std::fs::read_to_string("/proc/sys/kernel/osrelease")
        .ok()
        .map(|s| s.trim().to_string())
        .unwrap_or_default();

    let shell = std::env::var("SHELL")
        .ok()
        .and_then(|s| s.rsplit('/').next().map(|s| s.to_string()))
        .unwrap_or_default();

    let sys = crate::app::SYS.lock().unwrap();
    let total_b = sys.total_memory();
    let used_b = sys.used_memory();
    drop(sys);

    let bat = match sum.bat_pct {
        Some(p) => format!("{}%", p),
        None => "AC".to_string(),
    };

    NeoData {
        os: read_os_pretty(),
        host,
        kernel,
        uptime: sum.uptime.clone(),
        shell,
        resolution: format!("{}x{}", w, h),
        cpu: read_cpu_short(),
        gpu: read_gpu_short(),
        memory: format!("{} / {}", fmt_ram(used_b), fmt_ram(total_b)),
        bat,
    }
}
