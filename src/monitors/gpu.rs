use std::process::Command;
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant};

use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::app;

#[derive(Clone, Copy)]
struct GpuData {
    util_pct: f64,
    temp_c: f64,
    mem_used_mb: f64,
    mem_total_mb: f64,
}

struct CachedGpu {
    data: Option<GpuData>,
    timestamp: Instant,
}

/// Rolling GPU utilisation history, so the panel gets a real graph like btop
/// instead of a flat gauge. Fed from the same cached read.
const HIST_LEN: usize = 240;
static GPU_HISTORY: LazyLock<Mutex<([f64; HIST_LEN], usize)>> =
    LazyLock::new(|| Mutex::new(([0.0; HIST_LEN], 0)));

static GPU_CACHE: LazyLock<Mutex<CachedGpu>> = LazyLock::new(|| {
    Mutex::new(CachedGpu {
        data: None,
        timestamp: Instant::now() - Duration::from_secs(2), // start expired
    })
});

fn read_gpu_raw() -> Option<GpuData> {
    read_nvidia().or_else(read_amd).or_else(read_intel)
}

fn read_nvidia() -> Option<GpuData> {
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

fn read_amd() -> Option<GpuData> {
    use std::fs;
    // AMD exposes GPU busy % via sysfs
    let drm = fs::read_dir("/sys/class/drm").ok()?;
    for entry in drm.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if !name.starts_with("card") || name.contains('-') {
            continue;
        }
        let dev = entry.path().join("device");
        let util_path = dev.join("gpu_busy_percent");
        let util_pct = fs::read_to_string(&util_path)
            .ok()
            .and_then(|s| s.trim().parse::<f64>().ok())?;

        // Temp from hwmon
        let temp_c = fs::read_dir(dev.join("hwmon"))
            .ok()?
            .flatten()
            .find_map(|hwmon_entry| {
                fs::read_to_string(hwmon_entry.path().join("temp1_input"))
                    .ok()
                    .and_then(|s| s.trim().parse::<f64>().ok())
                    .map(|t| t / 1000.0) // millidegrees -> degrees
            })
            .unwrap_or(0.0);

        // AMD doesn't expose VRAM via sysfs reliably, approximate or leave 0
        return Some(GpuData {
            util_pct,
            temp_c,
            mem_used_mb: 0.0,
            mem_total_mb: 0.0,
        });
    }
    None
}

fn read_intel() -> Option<GpuData> {
    use std::fs;
    // Intel integrated GPUs expose utilization via /sys/class/drm/card*/gt/gt0/rps_cur_freq_mhz
    // and other metrics, but it's less standardized. Best-effort.
    let drm = fs::read_dir("/sys/class/drm").ok()?;
    for entry in drm.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if !name.starts_with("card") || name.contains('-') {
            continue;
        }
        let dev = entry.path().join("device");
        // Intel doesn't expose a direct busy_percent like AMD; skip or return minimal
        // Just check if it's an Intel GPU and return 0s as fallback
        if dev.join("vendor").exists() {
            if let Ok(vendor) = fs::read_to_string(dev.join("vendor")) {
                if vendor.trim() == "0x8086" {
                    // It's Intel, but no easy util read
                    return Some(GpuData {
                        util_pct: 0.0,
                        temp_c: 0.0,
                        mem_used_mb: 0.0,
                        mem_total_mb: 0.0,
                    });
                }
            }
        }
    }
    None
}

/// Cached GPU read. The render loop runs at ~125fps but `nvidia-smi` is a
/// subprocess spawn — without this cache it forks 125×/sec. Refresh at most
/// once per second; every other caller gets the cached value.
fn read_gpu() -> Option<GpuData> {
    let mut cache = GPU_CACHE.lock().unwrap();
    if cache_is_stale(cache.timestamp, Duration::from_secs(1)) {
        cache.data = read_gpu_raw();
        cache.timestamp = Instant::now();
    }
    cache.data
}

/// True when a cached value older than `ttl` should be refreshed.
fn cache_is_stale(timestamp: Instant, ttl: Duration) -> bool {
    timestamp.elapsed() >= ttl
}

/// GPU utilization % for the top-bar Summary — reuses the same 1s cache as the
/// GPU widget, so there's only ever one `nvidia-smi` spawn per second total.
pub fn util_pct() -> u64 {
    read_gpu().map(|g| g.util_pct as u64).unwrap_or(0)
}

pub fn render(f: &mut Frame, area: Rect, theme: &app::Theme) {
    let gpu_data = read_gpu();

    let gpu = match gpu_data {
        Some(g) => g,
        None => {
            f.render_widget(
                Paragraph::new(Line::from(Span::styled(
                    " GPU n/a",
                    Style::default().fg(theme.dim),
                ))),
                area,
            );
            return;
        }
    };

    // Record utilisation for the history graph.
    {
        let mut h = GPU_HISTORY.lock().unwrap();
        let idx = h.1;
        h.0[idx] = gpu.util_pct;
        h.1 = (idx + 1) % HIST_LEN;
    }

    let util_color = if gpu.util_pct > 95.0 {
        theme.red
    } else if gpu.util_pct > 80.0 {
        theme.yellow
    } else {
        theme.accent
    };

    let chunks = Layout::vertical([
        Constraint::Length(1), // info line
        Constraint::Min(1),    // util history graph
        Constraint::Length(1), // VRAM bar
    ])
    .split(area);

    let mem_pct = if gpu.mem_total_mb > 0.0 {
        gpu.mem_used_mb / gpu.mem_total_mb * 100.0
    } else {
        0.0
    };

    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(
                format!("{:>3.0}% ", gpu.util_pct),
                Style::default()
                    .fg(util_color)
                    .add_modifier(ratatui::style::Modifier::BOLD),
            ),
            Span::styled(
                format!("{}\u{00b0}C  ", gpu.temp_c as u64),
                Style::default().fg(theme.dim),
            ),
            Span::styled(
                format!("VRAM {:.0}/{:.0} MiB", gpu.mem_used_mb, gpu.mem_total_mb),
                Style::default().fg(theme.text),
            ),
        ])),
        chunks[0],
    );

    if chunks[1].height > 0 {
        let want = (chunks[1].width as usize * 2).min(HIST_LEN);
        let series: Vec<f64> = {
            let h = GPU_HISTORY.lock().unwrap();
            let idx = h.1;
            (0..want)
                .map(|i| h.0[(idx + HIST_LEN - 1 - i) % HIST_LEN])
                .rev()
                .collect()
        };
        let graph = crate::widgets::block_graph::BlockGraph::new(&series)
            .min(0.0)
            .max(100.0)
            .colors(theme.green, theme.yellow, theme.red);
        f.render_widget(graph, chunks[1]);
    }

    // VRAM as a compact block meter on the last row.
    let bar_w = chunks[2].width.saturating_sub(16) as usize;
    let filled = ((mem_pct / 100.0).clamp(0.0, 1.0) * bar_w as f64).round() as usize;
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("VRAM ", Style::default().fg(theme.dim)),
            Span::styled(
                "\u{25a0}".repeat(filled),
                Style::default().fg(theme.secondary),
            ),
            Span::styled(
                "\u{25a0}".repeat(bar_w.saturating_sub(filled)),
                Style::default().fg(theme.surface),
            ),
            Span::styled(
                format!(" {:>3.0}%", mem_pct),
                Style::default().fg(theme.secondary),
            ),
        ])),
        chunks[2],
    );
}
