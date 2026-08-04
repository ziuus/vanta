use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::app;

use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};
use std::time::Instant;

const HISTORY_LEN: usize = 40;

struct DiskState {
    prev_io: Option<DiskIoState>,
    prev_time: Option<Instant>,
    io_history: HashMap<String, Vec<(f64, f64)>>,
}

static DISK: LazyLock<Mutex<DiskState>> = LazyLock::new(|| {
    Mutex::new(DiskState {
        prev_io: None,
        prev_time: None,
        io_history: HashMap::new(),
    })
});

struct DiskIoState {
    reads: HashMap<String, u64>,
    writes: HashMap<String, u64>,
}

fn gauge_color(usage: f64, theme: &app::Theme) -> Color {
    if usage < 80.0 {
        theme.accent
    } else if usage < 95.0 {
        theme.yellow
    } else {
        theme.red
    }
}

fn read_disk_io() -> HashMap<String, (u64, u64)> {
    let content = std::fs::read_to_string("/proc/diskstats").unwrap_or_default();
    let mut result = HashMap::new();
    for line in content.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 14 {
            let name = parts[2].to_string();
            if name.starts_with("loop")
                || name.starts_with("ram")
                || name.starts_with("sr")
                || name.contains("dm-")
                || name.starts_with("fd")
                || name.starts_with("zram")
            {
                continue;
            }
            if let (Ok(sectors_read), Ok(sectors_written)) =
                (parts[5].parse::<u64>(), parts[9].parse::<u64>())
            {
                result.insert(name, (sectors_read * 512, sectors_written * 512));
            }
        }
    }
    result
}

/// Map mountpoint → block device name (e.g. "/" → "nvme0n1p2") from /proc/mounts.
fn read_mount_devices() -> HashMap<String, String> {
    let mut map = HashMap::new();
    if let Ok(content) = std::fs::read_to_string("/proc/mounts") {
        for line in content.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let dev = parts[0];
                let base = dev.rsplit('/').next().unwrap_or(dev);
                if base.starts_with("sd")
                    || base.starts_with("nvme")
                    || base.starts_with("hd")
                    || base.starts_with("vd")
                    || base.starts_with("xvd")
                    || base.starts_with("mmcblk")
                    || base.contains("dm-")
                {
                    map.insert(parts[1].to_string(), base.to_string());
                }
            }
        }
    }
    map
}

fn update_io_history() {
    let now = Instant::now();
    let current = read_disk_io();

    let mut disk = DISK.lock().unwrap();
    let elapsed = disk
        .prev_time
        .map(|t| now.duration_since(t).as_secs_f64())
        .unwrap_or(1.0);
    let elapsed = elapsed.max(0.1);

    let mut read_kbps = HashMap::new();
    let mut write_kbps = HashMap::new();

    if let Some(ref prev) = disk.prev_io {
        for (dev, &(cur_read, cur_write)) in &current {
            if let Some(&prev_read) = prev.reads.get(dev) {
                if cur_read >= prev_read {
                    read_kbps.insert(
                        dev.clone(),
                        (cur_read - prev_read) as f64 / 1024.0 / elapsed,
                    );
                }
            }
            if let Some(&prev_write) = prev.writes.get(dev) {
                if cur_write >= prev_write {
                    write_kbps.insert(
                        dev.clone(),
                        (cur_write - prev_write) as f64 / 1024.0 / elapsed,
                    );
                }
            }
        }
    }

    disk.prev_io = Some(DiskIoState {
        reads: current.iter().map(|(k, v)| (k.clone(), v.0)).collect(),
        writes: current.iter().map(|(k, v)| (k.clone(), v.1)).collect(),
    });
    disk.prev_time = Some(now);

    for (dev, &r) in &read_kbps {
        if let Some(w) = write_kbps.get(dev) {
            let entry = disk.io_history.entry(dev.clone()).or_default();
            entry.push((r, *w));
            if entry.len() > HISTORY_LEN {
                entry.remove(0);
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

fn get_current_rates(mount: &str) -> Option<(f64, f64)> {
    let devices = read_mount_devices();
    let device = devices.get(mount)?;
    let disk = DISK.lock().unwrap();
    if let Some(entries) = disk.io_history.get(device) {
        if let Some(&(r, w)) = entries.last() {
            return Some((r, w));
        }
    }
    None
}

pub fn render(f: &mut Frame, area: Rect, theme: &app::Theme) {
    let entries: Vec<_> = {
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
                let rates = get_current_rates(&mount);
                (mount, pct, rates)
            })
            .collect()
    };

    // Compact: each disk = 1 gauge with detailed label
    for (i, (mount, pct, rates)) in entries.iter().enumerate() {
        let y_offset = i as u16;
        if y_offset >= area.height {
            break;
        }
        let chunk_area = Rect::new(area.x, area.y + y_offset, area.width, 1);

        let color = gauge_color(*pct, theme);
        let stats = match rates {
            Some((r, w)) => format!("↓ {:>9}  ↑ {:>9}", fmt_rate(*r), fmt_rate(*w)),
            None => format!("↓ {:>9}  ↑ {:>9}", "--", "--"),
        };

        let label = mount.chars().take(8).collect::<String>();
        let label_fmt = format!("{:<5}", label);
        let pct_str = format!("{:>3.0}%", pct);
        let needed_w = label_fmt.len() + stats.len() + pct_str.len() + 3;
        let dots_w = chunk_area.width.saturating_sub(needed_w as u16) as usize;
        let dots = if dots_w > 0 {
            "·".repeat(dots_w)
        } else {
            String::new()
        };

        let line = ratatui::text::Line::from(vec![
            ratatui::text::Span::styled(format!("{} ", label_fmt), Style::default().fg(theme.dim)),
            ratatui::text::Span::styled(format!("{} ", stats), Style::default().fg(theme.text)),
            ratatui::text::Span::styled(dots, Style::default().fg(theme.dim)),
            ratatui::text::Span::styled(
                format!(" {}", pct_str),
                Style::default()
                    .fg(color)
                    .add_modifier(ratatui::style::Modifier::BOLD),
            ),
        ]);
        f.render_widget(Paragraph::new(line), chunk_area);
    }
}
