use std::fs;
use std::process::Command;
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant};

use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::monitors::history::History;
use crate::theme::Theme;
use crate::widgets::block_graph::BlockGraph;
use crate::widgets::meter;

#[derive(Clone, Debug)]
pub struct GpuData {
    pub name: String,
    pub util_pct: Option<f64>,
    pub temp_c: Option<f64>,
    pub mem_used_mb: Option<f64>,
    pub mem_total_mb: Option<f64>,
    pub freq_mhz: Option<u64>,
}

struct Cache {
    data: Option<GpuData>,
    stamp: Option<Instant>,
    /// Set once nvidia-smi is known to be missing or failing, so we don't
    /// keep forking it every second on machines without an NVIDIA card.
    nvidia_dead: bool,
}

static CACHE: LazyLock<Mutex<Cache>> = LazyLock::new(|| {
    Mutex::new(Cache {
        data: None,
        stamp: None,
        nvidia_dead: false,
    })
});
static HISTORY: LazyLock<Mutex<History<240>>> = LazyLock::new(|| Mutex::new(History::new()));

const TTL: Duration = Duration::from_secs(1);

pub fn snapshot() -> Option<GpuData> {
    CACHE.lock().unwrap().data.clone()
}

/// Utilisation for the header summary; None when the GPU has no telemetry.
pub fn util_pct() -> Option<f64> {
    snapshot().and_then(|g| g.util_pct)
}

/// Refresh at most once per second — nvidia-smi is a subprocess.
pub fn sample() {
    let mut c = CACHE.lock().unwrap();
    if c.stamp.is_some_and(|t| t.elapsed() < TTL) {
        return;
    }
    let data = if c.nvidia_dead {
        None
    } else {
        let d = read_nvidia();
        if d.is_none() {
            c.nvidia_dead = true;
        }
        d
    };
    let data = data.or_else(read_amd).or_else(read_intel);
    if let Some(u) = data.as_ref().and_then(|d| d.util_pct) {
        HISTORY.lock().unwrap().push(u);
    }
    c.data = data;
    c.stamp = Some(Instant::now());
}

fn read_nvidia() -> Option<GpuData> {
    let out = Command::new("nvidia-smi")
        .args([
            "--query-gpu=name,utilization.gpu,temperature.gpu,memory.used,memory.total,clocks.sm",
            "--format=csv,noheader,nounits",
        ])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&out.stdout);
    let line = s.lines().next()?;
    let parts: Vec<&str> = line.split(',').map(str::trim).collect();
    if parts.len() < 6 {
        return None;
    }
    let num = |i: usize| parts[i].parse::<f64>().ok();
    Some(GpuData {
        name: short_name(parts[0]),
        util_pct: num(1),
        temp_c: num(2),
        mem_used_mb: num(3),
        mem_total_mb: num(4),
        freq_mhz: num(5).map(|v| v as u64),
    })
}

fn drm_cards() -> Vec<std::path::PathBuf> {
    let Ok(drm) = fs::read_dir("/sys/class/drm") else {
        return Vec::new();
    };
    let mut cards: Vec<_> = drm
        .flatten()
        .filter(|e| {
            let n = e.file_name().to_string_lossy().to_string();
            n.starts_with("card") && !n.contains('-')
        })
        .map(|e| e.path())
        .collect();
    cards.sort();
    cards
}

fn read_u64(p: std::path::PathBuf) -> Option<u64> {
    fs::read_to_string(p).ok()?.trim().parse().ok()
}

fn read_amd() -> Option<GpuData> {
    for card in drm_cards() {
        let dev = card.join("device");
        let Some(util) = read_u64(dev.join("gpu_busy_percent")) else {
            continue;
        };
        let temp_c = fs::read_dir(dev.join("hwmon")).ok().and_then(|d| {
            d.flatten()
                .find_map(|h| read_u64(h.path().join("temp1_input")))
                .map(|t| t as f64 / 1000.0)
        });
        let mem_used = read_u64(dev.join("mem_info_vram_used")).map(|b| b as f64 / 1_048_576.0);
        let mem_total = read_u64(dev.join("mem_info_vram_total")).map(|b| b as f64 / 1_048_576.0);
        let freq = fs::read_to_string(dev.join("pp_dpm_sclk"))
            .ok()
            .and_then(|s| {
                s.lines()
                    .find(|l| l.ends_with('*'))
                    .and_then(|l| l.split_whitespace().nth(1))
                    .and_then(|v| v.trim_end_matches("Mhz").parse().ok())
            });
        return Some(GpuData {
            name: "AMD Radeon".to_string(),
            util_pct: Some(util as f64),
            temp_c,
            mem_used_mb: mem_used,
            mem_total_mb: mem_total,
            freq_mhz: freq,
        });
    }
    None
}

fn read_intel() -> Option<GpuData> {
    for card in drm_cards() {
        let dev = card.join("device");
        let vendor = fs::read_to_string(dev.join("vendor")).ok()?;
        if vendor.trim() != "0x8086" {
            continue;
        }
        // i915 exposes the current GT clock; there is no busy% without
        // perf counters, so utilisation stays None and the panel says so.
        let freq = read_u64(card.join("gt/gt0/rps_cur_freq_mhz"))
            .or_else(|| read_u64(card.join("gt_cur_freq_mhz")));
        return Some(GpuData {
            name: "Intel iGPU".to_string(),
            util_pct: None,
            temp_c: None,
            mem_used_mb: None,
            mem_total_mb: None,
            freq_mhz: freq,
        });
    }
    None
}

fn short_name(model: &str) -> String {
    model
        .replace("NVIDIA GeForce ", "")
        .replace("NVIDIA ", "")
        .replace("AMD Radeon ", "")
        .replace("Intel Corporation ", "")
        .replace(" Graphics", "")
        .trim()
        .to_string()
}

/// Human-readable GPU name for the system panel; empty when none is detected.
pub fn name() -> String {
    snapshot().map(|g| g.name).unwrap_or_default()
}

fn centered_note(f: &mut Frame, area: Rect, text: &str, theme: &Theme) {
    let y = area.y + area.height.saturating_sub(1) / 2;
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            text,
            Style::default().fg(theme.dim),
        )))
        .alignment(ratatui::layout::Alignment::Center),
        Rect::new(area.x, y, area.width, 1),
    );
}

pub fn render(f: &mut Frame, area: Rect, theme: &Theme) {
    if area.height < 1 {
        return;
    }
    let Some(gpu) = snapshot() else {
        centered_note(f, area, "no GPU detected", theme);
        return;
    };

    let Some(util) = gpu.util_pct else {
        let freq = gpu
            .freq_mhz
            .map(|m| format!(" · {} MHz", m))
            .unwrap_or_default();
        centered_note(
            f,
            area,
            &format!("{}{} · no utilisation telemetry", gpu.name, freq),
            theme,
        );
        return;
    };

    let has_vram = gpu.mem_total_mb.is_some_and(|t| t > 0.0);
    let chunks = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(1),
        Constraint::Length(if has_vram { 1 } else { 0 }),
    ])
    .split(area);

    let mut head = vec![
        Span::styled(
            format!("{:>3.0}%", util),
            Style::default()
                .fg(theme.usage(util))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(format!("  {}", gpu.name), Style::default().fg(theme.text)),
    ];
    if let Some(t) = gpu.temp_c {
        head.push(Span::styled(
            format!("  {:.0}°C", t),
            Style::default().fg(theme.temp(t)),
        ));
    }
    if let Some(m) = gpu.freq_mhz {
        head.push(Span::styled(
            format!("  {} MHz", m),
            Style::default().fg(theme.dim),
        ));
    }
    f.render_widget(Paragraph::new(Line::from(head)), chunks[0]);

    let hist = HISTORY.lock().unwrap().recent(chunks[1].width as usize);
    f.render_widget(
        BlockGraph::new(&hist)
            .max(100.0)
            .colors(theme.accent, theme.yellow, theme.red),
        chunks[1],
    );

    if has_vram {
        let used = gpu.mem_used_mb.unwrap_or(0.0);
        let total = gpu.mem_total_mb.unwrap_or(1.0);
        let pct = used / total * 100.0;
        let stats = format!("{:.0}/{:.0} MiB", used, total);
        let pct_s = format!("{:>3.0}%", pct);
        let bar_w =
            (chunks[2].width as usize).saturating_sub(5 + stats.len() + 2 + pct_s.len() + 1);
        let (on, off) = meter::track(pct / 100.0, bar_w);
        f.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled("VRAM ", Style::default().fg(theme.dim)),
                Span::styled(format!("{}  ", stats), Style::default().fg(theme.text)),
                Span::styled(on, Style::default().fg(theme.secondary)),
                Span::styled(off, Style::default().fg(theme.surface)),
                Span::styled(format!(" {}", pct_s), Style::default().fg(theme.secondary)),
            ])),
            chunks[2],
        );
    }
}
