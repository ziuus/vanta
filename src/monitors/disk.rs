use std::collections::HashMap;
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

const HIST: usize = 120;
const MOUNT_TTL: Duration = Duration::from_secs(5);

#[derive(Clone, Debug)]
pub struct Mount {
    pub path: String,
    pub device: String,
    pub used: u64,
    pub total: u64,
}

impl Mount {
    pub fn pct(&self) -> f64 {
        if self.total == 0 {
            0.0
        } else {
            self.used as f64 / self.total as f64 * 100.0
        }
    }
}

struct DevIo {
    read: u64,
    write: u64,
    read_kbps: f64,
    write_kbps: f64,
    hist: History<HIST>,
}

struct State {
    mounts: Vec<Mount>,
    mounts_stamp: Option<Instant>,
    io: HashMap<String, DevIo>,
    io_stamp: Option<Instant>,
}

static STATE: LazyLock<Mutex<State>> = LazyLock::new(|| {
    Mutex::new(State {
        mounts: Vec::new(),
        mounts_stamp: None,
        io: HashMap::new(),
        io_stamp: None,
    })
});

pub fn mounts() -> Vec<Mount> {
    STATE.lock().unwrap().mounts.clone()
}

/// Used % of the root filesystem for the header summary.
pub fn root_pct() -> Option<f64> {
    STATE
        .lock()
        .unwrap()
        .mounts
        .iter()
        .find(|m| m.path == "/")
        .map(Mount::pct)
}

fn is_real_mount(fs: &str, dev: &str, mount: &str) -> bool {
    !fs.is_empty()
        && !matches!(
            fs,
            "tmpfs" | "devtmpfs" | "squashfs" | "overlay" | "efivarfs" | "fuse.portal"
        )
        && !dev.contains("loop")
        && !mount.starts_with("/run")
        && !mount.starts_with("/snap")
        && !mount.starts_with("/tmp/.mount")
        && !mount.starts_with("/var/lib/docker")
        && !mount.contains("/waydroid")
        && !mount.contains("/efivars")
}

fn collect_mounts() -> Vec<Mount> {
    let devices = mount_devices();
    let disks = sysinfo::Disks::new_with_refreshed_list();
    let mut list: Vec<Mount> = disks
        .iter()
        .filter_map(|d| {
            let fs = d.file_system().to_str().unwrap_or("");
            let dev = d.name().to_str().unwrap_or("");
            let path = d.mount_point().to_string_lossy().to_string();
            if !is_real_mount(fs, dev, &path) || d.total_space() == 0 {
                return None;
            }
            Some(Mount {
                device: devices.get(&path).cloned().unwrap_or_default(),
                path,
                used: d.total_space().saturating_sub(d.available_space()),
                total: d.total_space(),
            })
        })
        .collect();
    list.sort_by(|a, b| a.path.cmp(&b.path));
    list.dedup_by(|a, b| a.path == b.path);
    list
}

/// mountpoint → block device (e.g. "/" → "nvme0n1p2"), from /proc/mounts.
fn mount_devices() -> HashMap<String, String> {
    let mut map = HashMap::new();
    let Ok(content) = std::fs::read_to_string("/proc/mounts") else {
        return map;
    };
    for line in content.lines() {
        let mut p = line.split_whitespace();
        let (Some(dev), Some(mount)) = (p.next(), p.next()) else {
            continue;
        };
        let base = dev.rsplit('/').next().unwrap_or(dev);
        if ["sd", "nvme", "hd", "vd", "xvd", "mmcblk", "dm-"]
            .iter()
            .any(|pre| base.starts_with(pre))
        {
            map.insert(mount.to_string(), base.to_string());
        }
    }
    map
}

/// Whole-device (not partition) counters from /proc/diskstats, in bytes.
fn read_diskstats() -> HashMap<String, (u64, u64)> {
    let content = std::fs::read_to_string("/proc/diskstats").unwrap_or_default();
    let mut out = HashMap::new();
    for line in content.lines() {
        let p: Vec<&str> = line.split_whitespace().collect();
        if p.len() < 14 {
            continue;
        }
        let name = p[2];
        if ["loop", "ram", "sr", "fd", "zram", "dm-"]
            .iter()
            .any(|pre| name.starts_with(pre))
        {
            continue;
        }
        if let (Ok(r), Ok(w)) = (p[5].parse::<u64>(), p[9].parse::<u64>()) {
            out.insert(name.to_string(), (r * 512, w * 512));
        }
    }
    out
}

pub fn sample() {
    // Everything slow (statvfs on every mount, /proc reads) happens before
    // the lock so renders never stall behind it.
    let stale = STATE
        .lock()
        .unwrap()
        .mounts_stamp
        .is_none_or(|t| t.elapsed() > MOUNT_TTL);
    let fresh_mounts = stale.then(collect_mounts);
    let cur = read_diskstats();
    let now = Instant::now();

    let mut st = STATE.lock().unwrap();
    if let Some(m) = fresh_mounts {
        st.mounts = m;
        st.mounts_stamp = Some(now);
    }
    let dt = st
        .io_stamp
        .map(|t| now.duration_since(t).as_secs_f64())
        .unwrap_or(0.0);
    for (dev, (r, w)) in cur {
        let e = st.io.entry(dev).or_insert(DevIo {
            read: r,
            write: w,
            read_kbps: 0.0,
            write_kbps: 0.0,
            hist: History::new(),
        });
        if dt > 0.05 {
            e.read_kbps = r.saturating_sub(e.read) as f64 / 1024.0 / dt;
            e.write_kbps = w.saturating_sub(e.write) as f64 / 1024.0 / dt;
            let sum = e.read_kbps + e.write_kbps;
            e.hist.push(sum);
        }
        e.read = r;
        e.write = w;
    }
    st.io_stamp = Some(now);
}

/// Strip the partition suffix so "nvme0n1p2" → "nvme0n1", "sda3" → "sda".
fn parent_device(part: &str) -> String {
    if let Some(idx) = part.rfind('p') {
        if part.starts_with("nvme") || part.starts_with("mmcblk") {
            return part[..idx].to_string();
        }
    }
    part.trim_end_matches(|c: char| c.is_ascii_digit())
        .to_string()
}

fn io_for(st: &State, mount: &Mount) -> Option<(f64, f64, Vec<f64>, usize)> {
    let dev = st
        .io
        .get(&mount.device)
        .or_else(|| st.io.get(&parent_device(&mount.device)))?;
    Some((
        dev.read_kbps,
        dev.write_kbps,
        dev.hist.recent(HIST),
        dev.hist.recent(HIST).len(),
    ))
}

/// Monitor page: per-mount header, IO history graph, capacity meter.
pub fn render(f: &mut Frame, area: Rect, theme: &Theme) {
    if area.height < 2 {
        return;
    }
    let st = STATE.lock().unwrap();
    if st.mounts.is_empty() {
        return;
    }
    // Each mount needs header + meter (2 rows) plus at least one graph row.
    let shown = st.mounts.len().min((area.height as usize / 3).max(1));
    let slots = Layout::vertical(vec![Constraint::Ratio(1, shown as u32); shown]).split(area);

    for (slot, m) in slots.iter().zip(st.mounts.iter()) {
        if slot.height < 2 {
            continue;
        }
        let rows = Layout::vertical([
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(*slot);

        let io = io_for(&st, m);
        let stats = match &io {
            Some((r, w, _, _)) => {
                format!("↓ {:>9}  ↑ {:>9}", meter::fmt_kbps(*r), meter::fmt_kbps(*w))
            }
            None => String::from("↓        --  ↑        --"),
        };
        f.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(
                    format!("{:<8}", meter::ellipsize(&m.path, 8)),
                    Style::default().fg(theme.text).add_modifier(Modifier::BOLD),
                ),
                Span::styled(stats, Style::default().fg(theme.dim)),
            ])),
            rows[0],
        );

        if rows[1].height > 0 {
            if let Some((_, _, series, n)) = &io {
                if *n > 0 {
                    let peak = series.iter().copied().fold(1.0f64, f64::max);
                    let want = series.len().saturating_sub(rows[1].width as usize);
                    f.render_widget(
                        BlockGraph::new(&series[want..]).max(peak).colors(
                            theme.secondary,
                            theme.secondary,
                            theme.secondary,
                        ),
                        rows[1],
                    );
                }
            }
        }

        let pct = m.pct();
        let c = theme.usage(pct);
        let tail = format!(
            " {:>3.0}%  {} / {}",
            pct,
            meter::fmt_bytes(m.used),
            meter::fmt_bytes(m.total)
        );
        let bar_w = (rows[2].width as usize).saturating_sub(tail.len());
        let (on, off) = meter::track(pct / 100.0, bar_w);
        f.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(on, Style::default().fg(c)),
                Span::styled(off, Style::default().fg(theme.surface)),
                Span::styled(tail, Style::default().fg(c).add_modifier(Modifier::BOLD)),
            ])),
            rows[2],
        );
    }
}

/// Dashboard: one compact capacity row per mount.
pub fn render_storage(f: &mut Frame, area: Rect, theme: &Theme) {
    if area.height < 1 || area.width < 24 {
        return;
    }
    let mounts = mounts();
    if mounts.is_empty() {
        return;
    }
    let wide = area.width >= 44;
    let lines: Vec<Line> = mounts
        .iter()
        .take(area.height as usize)
        .map(|m| {
            let pct = m.pct();
            let c = theme.usage(pct);
            let label = format!("{:<9}", meter::ellipsize(&m.path, 9));
            let pct_s = format!(" {:>3.0}%", pct);
            let size = if wide {
                format!(
                    "  {:>6} / {:<6}",
                    meter::fmt_bytes(m.used),
                    meter::fmt_bytes(m.total)
                )
            } else {
                String::new()
            };
            let bar_w = (area.width as usize)
                .saturating_sub(label.len() + pct_s.len() + size.len())
                .max(4);
            let (on, off) = meter::track(pct / 100.0, bar_w);
            Line::from(vec![
                Span::styled(label, Style::default().fg(theme.dim)),
                Span::styled(on, Style::default().fg(c)),
                Span::styled(off, Style::default().fg(theme.surface)),
                Span::styled(pct_s, Style::default().fg(c).add_modifier(Modifier::BOLD)),
                Span::styled(size, Style::default().fg(theme.dim)),
            ])
        })
        .collect();
    let top = area.height.saturating_sub(lines.len() as u16) / 2;
    f.render_widget(
        Paragraph::new(lines),
        Rect::new(area.x, area.y + top, area.width, area.height - top),
    );
}
