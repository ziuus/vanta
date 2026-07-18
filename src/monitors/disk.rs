use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::app;

use std::collections::HashMap;
use std::time::Instant;

const HISTORY_LEN: usize = 40;

static mut PREV_DISK_IO: Option<DiskIoState> = None;
static mut PREV_DISK_TIME: Option<Instant> = None;
static mut IO_HISTORY: Option<HashMap<String, Vec<(f64, f64)>>> = None;

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
            if name.starts_with("loop") || name.starts_with("ram") || name.starts_with("sr")
                || name.contains("dm-") || (name.len() > 1 && name.chars().any(|c| c.is_ascii_digit()))
            {
                continue;
            }
            if let (Ok(sectors_read), Ok(sectors_written)) = (parts[5].parse::<u64>(), parts[9].parse::<u64>()) {
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

        PREV_DISK_IO = Some(DiskIoState {
            reads: current.iter().map(|(k, v)| (k.clone(), v.0)).collect(),
            writes: current.iter().map(|(k, v)| (k.clone(), v.1)).collect(),
        });
        PREV_DISK_TIME = Some(now);

        #[allow(static_mut_refs)]
        let hist = IO_HISTORY.get_or_insert_with(HashMap::new);
        for (dev, &r) in &read_kbps {
            if let Some(w) = write_kbps.get(dev) {
                let entry = hist.entry(dev.clone()).or_default();
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
                let mount_name = mount.rsplit('/').next().unwrap_or("");
                let (read_rate, write_rate) = if !mount_name.is_empty() {
                    get_current_rates(mount_name)
                } else {
                    (0.0, 0.0)
                };
                (mount, pct, read_rate, write_rate)
            })
            .collect()
    };

    // Compact: each disk = 1 gauge with detailed label
    for (i, (mount, pct, read_rate, write_rate)) in entries.iter().enumerate() {
        let y_offset = i as u16;
        let chunk = Layout::vertical([Constraint::Length(1)])
            .split(Rect::new(area.x, area.y + y_offset, area.width, 1));

        let color = gauge_color(*pct, theme);
        let stats = format!("↓ {:>9}  ↑ {:>9}", fmt_rate(*read_rate), fmt_rate(*write_rate));
        let bar_line = crate::widgets::bar::draw_premium_bar(
            &mount.chars().take(8).collect::<String>(),
            8,
            &stats,
            24,
            *pct / 100.0,
            color,
            theme.surface,
            chunk[0].width,
        );
        f.render_widget(Paragraph::new(bar_line), chunk[0]);
    }
}
