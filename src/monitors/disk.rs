use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Gauge, Paragraph, Sparkline};
use ratatui::Frame;

use crate::app;

use std::collections::HashMap;
use std::time::Instant;

const HISTORY_LEN: usize = 40;

static mut PREV_DISK_IO: Option<DiskIoState> = None;
static mut PREV_DISK_TIME: Option<Instant> = None;
static mut IO_HISTORY: Option<HashMap<String, Vec<(f64, f64)>>> = None; // dev -> [(read_kbps, write_kbps)]

struct DiskIoState {
    reads: HashMap<String, u64>,
    writes: HashMap<String, u64>,
}

fn gauge_color(usage: f64, theme: &app::Theme) -> Color {
    if usage < 50.0 { theme.green }
    else if usage < 80.0 { theme.yellow }
    else { theme.red }
}

fn read_disk_io() -> HashMap<String, (u64, u64)> {
    let content = std::fs::read_to_string("/proc/diskstats").unwrap_or_default();
    let mut result = HashMap::new();
    for line in content.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 14 {
            let name = parts[2].to_string();
            // Skip partitions, loop, ram, etc.
            if name.starts_with("loop") || name.starts_with("ram") || name.starts_with("sr")
                || name.contains("dm-") || (name.len() > 1 && name.chars().any(|c| c.is_ascii_digit()))
            {
                continue;
            }
            if let (Ok(sectors_read), Ok(sectors_written)) = (parts[5].parse::<u64>(), parts[9].parse::<u64>()) {
                // Each sector = 512 bytes
                result.insert(name, (sectors_read * 512, sectors_written * 512));
            }
        }
    }
    result
}

fn update_io_history() {
    let now = Instant::now();
    let current = read_disk_io();

    unsafe {
        let elapsed = PREV_DISK_TIME
            .map(|t| now.duration_since(t).as_secs_f64())
            .unwrap_or(1.0);
        let elapsed = elapsed.max(0.1);

        let mut read_kbps = HashMap::new();
        let mut write_kbps = HashMap::new();

        if let Some(ref prev) = PREV_DISK_IO {
            for (dev, &(cur_read, cur_write)) in &current {
                if let Some(&prev_read) = prev.reads.get(dev) {
                    if cur_read >= prev_read {
                        read_kbps.insert(dev.clone(), (cur_read - prev_read) as f64 / 1024.0 / elapsed);
                    }
                }
                if let Some(&prev_write) = prev.writes.get(dev) {
                    if cur_write >= prev_write {
                        write_kbps.insert(dev.clone(), (cur_write - prev_write) as f64 / 1024.0 / elapsed);
                    }
                }
            }
        }

        // Update prev state
        PREV_DISK_IO = Some(DiskIoState {
            reads: current.iter().map(|(k, v)| (k.clone(), v.0)).collect(),
            writes: current.iter().map(|(k, v)| (k.clone(), v.1)).collect(),
        });
        PREV_DISK_TIME = Some(now);

        // Push into history
        #[allow(static_mut_refs)]
        let hist = IO_HISTORY.get_or_insert_with(HashMap::new);
        for (dev, &r) in &read_kbps {
            if let Some(w) = write_kbps.get(dev) {
                let entry = hist.entry(dev.clone()).or_insert_with(Vec::new);
                entry.push((r, *w));
                if entry.len() > HISTORY_LEN {
                    entry.remove(0);
                }
            }
        }
    }
}

fn fmt_rate(kbps: f64) -> String {
    if kbps > 1024.0 {
        format!("{:.1} MB/s", kbps / 1024.0)
    } else {
        format!("{:.1} KB/s", kbps)
    }
}

/// Get the current I/O rates from the history (most recent entry)
fn get_current_rates(dev: &str) -> (f64, f64) {
    unsafe {
        if let Some(ref hist) = IO_HISTORY {
            if let Some(entries) = hist.get(dev) {
                if let Some(&(r, w)) = entries.last() {
                    return (r, w);
                }
            }
        }
    }
    (0.0, 0.0)
}

/// Per-disk data used for rendering, collected either from live data or demo values.
struct DiskEntry {
    mount: String,
    total: u64,
    used: u64,
    pct: f64,
    read_rate: f64,
    write_rate: f64,
}

pub fn render(f: &mut Frame, area: Rect, theme: &app::Theme, demo: bool) {
    // Collect entries: demo hardcoded data or real system data
    let entries: Vec<DiskEntry> = if demo {
        vec![
            DiskEntry {
                mount: "/".to_string(),
                total: 512 * 1024 * 1024 * 1024, // 512 GiB
                used: 256 * 1024 * 1024 * 1024,   // 256 GiB (label shows this, gauge uses pct)
                pct: 45.0,
                read_rate: 45.0 * 1024.0,   // 45 MB/s in KB/s
                write_rate: 12.0 * 1024.0,  // 12 MB/s in KB/s
            },
            DiskEntry {
                mount: "/home".to_string(),
                total: 512 * 1024 * 1024 * 1024,  // 512 GiB
                used: 320 * 1024 * 1024 * 1024,    // 320 GiB
                pct: 62.0,
                read_rate: 28.0 * 1024.0,   // 28 MB/s in KB/s
                write_rate: 6.5 * 1024.0,   // 6.5 MB/s in KB/s
            },
        ]
    } else {
        let disks = sysinfo::Disks::new_with_refreshed_list();
        let physical: Vec<_> = disks
            .iter()
            .filter(|d| {
                let fs = d.file_system().to_str().unwrap_or("");
                let name = d.name().to_str().unwrap_or("");
                !fs.is_empty()
                    && !name.contains("loop")
                    && !name.contains("squashfs")
                    && !name.contains("tmpfs")
                    && !name.contains("devtmpfs")
                    && !name.contains("overlay")
            })
            .collect();

        if physical.is_empty() {
            f.render_widget(
                Paragraph::new("  —").style(Style::default().bg(theme.surface)),
                area,
            );
            return;
        }

        let shown = physical.len().min(2);

        // Compute I/O rates from real /proc/diskstats data
        update_io_history();

        physical
            .iter()
            .take(shown)
            .map(|disk| {
                let total = disk.total_space();
                let available = disk.available_space();
                let used = total - available;
                let pct = if total > 0 {
                    (used as f64 / total as f64) * 100.0
                } else {
                    0.0
                };
                let mount = disk.mount_point().to_str().unwrap_or("?").to_string();
                let mount_name = mount.rsplit('/').next().unwrap_or("");
                let (read_rate, write_rate) = if !mount_name.is_empty() {
                    get_current_rates(mount_name)
                } else {
                    (0.0, 0.0)
                };
                DiskEntry {
                    mount,
                    total,
                    used,
                    pct,
                    read_rate,
                    write_rate,
                }
            })
            .collect()
    };

    // ── Shared rendering loop for both demo and real modes ──
    for (i, entry) in entries.iter().enumerate() {
        let mount = &entry.mount;
        let pct = entry.pct;

        // Each disk gets 4 rows: label, gauge, I/O rates, sparkline
        let y_offset = i as u16 * 4;
        let chunk = Layout::vertical([
            Constraint::Length(1), // label
            Constraint::Length(1), // gauge
            Constraint::Length(1), // I/O
            Constraint::Length(1), // sparkline
        ])
        .split(Rect::new(area.x, area.y + y_offset, area.width, 4));

        // ── Label ──
        let label = if demo {
            let total_gib = entry.total / (1024 * 1024 * 1024);
            let used_gib = entry.used / (1024 * 1024 * 1024);
            format!(" {} {}% ({}GiB/{}GiB)", mount, pct as u16, used_gib, total_gib)
        } else {
            format!(" {} {}%", mount, pct as u16)
        };
        f.render_widget(
            Paragraph::new(Line::from(Span::styled(
                label,
                Style::default().fg(theme.text),
            ))),
            chunk[0],
        );

        // ── Gauge ──
        let color = gauge_color(pct, theme);
        let gauge = Gauge::default()
            .gauge_style(Style::default().fg(color).bg(theme.surface))
            .percent(pct as u16)
            .label(String::new());
        f.render_widget(gauge, chunk[1]);

        // ── I/O rates ──
        f.render_widget(
            Paragraph::new(Line::from(Span::styled(
                format!(
                    " ↓ {}  ↑ {}",
                    fmt_rate(entry.read_rate),
                    fmt_rate(entry.write_rate)
                ),
                Style::default().fg(theme.dim),
            ))),
            chunk[2],
        );

        // ── Sparkline ──
        if demo {
            // Stable flat sparkline for demo mode
            let spark_data: Vec<u64> = vec![50; HISTORY_LEN];
            let spark_area = Layout::horizontal([
                Constraint::Ratio(1, 2),
                Constraint::Ratio(1, 2),
            ])
            .split(chunk[3]);

            f.render_widget(
                Sparkline::default()
                    .data(&spark_data)
                    .style(Style::default().fg(theme.green)),
                spark_area[0],
            );
            f.render_widget(
                Sparkline::default()
                    .data(&spark_data)
                    .style(Style::default().fg(theme.green)),
                spark_area[1],
            );
        } else {
            let mount_name = mount.rsplit('/').next().unwrap_or("");
            if !mount_name.is_empty() {
                unsafe {
                    if let Some(ref hist) = IO_HISTORY {
                        if let Some(hist_entries) = hist.get(mount_name) {
                            if hist_entries.len() > 1 {
                                // Normalize: find max across both read/write
                                let max_val = hist_entries
                                    .iter()
                                    .map(|&(r, w)| r.max(w))
                                    .fold(1.0f64, |a, b| a.max(b));
                                let max_val = max_val.max(1.0);

                                let read_data: Vec<u64> = hist_entries
                                    .iter()
                                    .map(|&(r, _)| (r / max_val * 100.0) as u64)
                                    .collect();
                                let write_data: Vec<u64> = hist_entries
                                    .iter()
                                    .map(|&(_, w)| (w / max_val * 100.0) as u64)
                                    .collect();

                                let spark_area = Layout::horizontal([
                                    Constraint::Ratio(1, 2),
                                    Constraint::Ratio(1, 2),
                                ])
                                .split(chunk[3]);

                                let read_color = if entry.read_rate > 50000.0 {
                                    theme.red
                                } else if entry.read_rate > 10000.0 {
                                    theme.yellow
                                } else {
                                    theme.green
                                };

                                let write_color = if entry.write_rate > 50000.0 {
                                    theme.red
                                } else if entry.write_rate > 10000.0 {
                                    theme.yellow
                                } else {
                                    theme.green
                                };

                                f.render_widget(
                                    Sparkline::default()
                                        .data(&read_data)
                                        .style(Style::default().fg(read_color)),
                                    spark_area[0],
                                );
                                f.render_widget(
                                    Sparkline::default()
                                        .data(&write_data)
                                        .style(Style::default().fg(write_color)),
                                    spark_area[1],
                                );
                            }
                        }
                    }
                }
            }
        }
    }
}
