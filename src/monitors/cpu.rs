use std::fs;
use std::sync::{LazyLock, Mutex};

use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::monitors::history::History;
use crate::theme::Theme;
use crate::widgets::meter;

#[derive(Clone, Default)]
pub struct CpuSnapshot {
    pub usage: f32,
    pub user_pct: f32,
    pub sys_pct: f32,
    pub iowait_pct: f32,
    pub cores: Vec<f32>,
    pub load: (f64, f64, f64),
    pub freq_mhz: u64,
    /// Per-sensor temps from hwmon (package first when the driver reports one).
    pub temps: Vec<f64>,
}

impl CpuSnapshot {
    /// Hottest sensor, or None when no sensor is available.
    pub fn max_temp(&self) -> Option<f64> {
        self.temps.iter().copied().reduce(f64::max)
    }
}

static SNAP: LazyLock<Mutex<CpuSnapshot>> = LazyLock::new(|| Mutex::new(CpuSnapshot::default()));
static HISTORY_USAGE: LazyLock<Mutex<History<240>>> = LazyLock::new(|| Mutex::new(History::new()));
static HISTORY_USER: LazyLock<Mutex<History<240>>> = LazyLock::new(|| Mutex::new(History::new()));
static HISTORY_SYS: LazyLock<Mutex<History<240>>> = LazyLock::new(|| Mutex::new(History::new()));
static HISTORY_IOWAIT: LazyLock<Mutex<History<240>>> = LazyLock::new(|| Mutex::new(History::new()));

pub fn snapshot() -> CpuSnapshot {
    SNAP.lock().unwrap().clone()
}

/// Called once per tick with a freshly refreshed sysinfo handle.
static PREV_STAT: Mutex<Option<(f64, f64, f64, f64)>> = Mutex::new(None); // (user, sys, iowait, total)

pub fn sample(sys: &sysinfo::System) {
    let mut user_pct = 0.0;
    let mut sys_pct = 0.0;
    let mut iowait_pct = 0.0;

    if let Ok(stat) = fs::read_to_string("/proc/stat") {
        if let Some(line) = stat.lines().find(|l| l.starts_with("cpu ")) {
            let p: Vec<f64> = line
                .split_whitespace()
                .skip(1)
                .filter_map(|s| s.parse().ok())
                .collect();
            if p.len() >= 8 {
                let user = p[0] + p[1];
                let sys_val = p[2] + p[5] + p[6];
                let iowait = p[4];
                let total = p.iter().sum::<f64>();

                let mut prev = PREV_STAT.lock().unwrap();
                if let Some((p_user, p_sys, p_iowait, p_total)) = *prev {
                    let d_total = total - p_total;
                    if d_total > 0.0 {
                        user_pct = ((user - p_user) / d_total * 100.0) as f32;
                        sys_pct = ((sys_val - p_sys) / d_total * 100.0) as f32;
                        iowait_pct = ((iowait - p_iowait) / d_total * 100.0) as f32;
                    }
                }
                *prev = Some((user, sys_val, iowait, total));
            }
        }
    }

    let la = sysinfo::System::load_average();
    let snap = CpuSnapshot {
        usage: sys.global_cpu_usage(),
        user_pct,
        sys_pct,
        iowait_pct,
        cores: sys.cpus().iter().map(|c| c.cpu_usage()).collect(),
        load: (la.one, la.five, la.fifteen),
        freq_mhz: sys.cpus().iter().map(|c| c.frequency()).max().unwrap_or(0),
        temps: read_core_temps(),
    };
    HISTORY_USAGE.lock().unwrap().push(snap.usage as f64);
    HISTORY_USER.lock().unwrap().push(snap.user_pct as f64);
    HISTORY_SYS.lock().unwrap().push(snap.sys_pct as f64);
    HISTORY_IOWAIT.lock().unwrap().push(snap.iowait_pct as f64);
    *SNAP.lock().unwrap() = snap;
}

fn read_core_temps() -> Vec<f64> {
    let Ok(hwmon_dir) = fs::read_dir("/sys/class/hwmon/") else {
        return Vec::new();
    };
    for hwmon_entry in hwmon_dir.flatten() {
        let Ok(name) = fs::read_to_string(hwmon_entry.path().join("name")) else {
            continue;
        };
        if !matches!(name.trim(), "coretemp" | "k10temp" | "zenpower") {
            continue;
        }
        let mut temps: Vec<(usize, f64)> = Vec::new();
        if let Ok(temp_dir) = fs::read_dir(hwmon_entry.path()) {
            for te in temp_dir.flatten() {
                let fname = te.file_name().to_string_lossy().to_string();
                let Some(num) = fname
                    .strip_prefix("temp")
                    .and_then(|s| s.strip_suffix("_input"))
                    .and_then(|s| s.parse::<usize>().ok())
                else {
                    continue;
                };
                if let Some(v) = fs::read_to_string(te.path())
                    .ok()
                    .and_then(|v| v.trim().parse::<f64>().ok())
                {
                    temps.push((num, v / 1000.0));
                }
            }
        }
        temps.sort_by_key(|(idx, _)| *idx);
        return temps.into_iter().map(|(_, t)| t).collect();
    }
    Vec::new()
}

/// One row of per-core load: each core gets an equal slice filled with a
/// block whose height and colour track its usage, e.g. `▂▂ ▇▇ ▁▁ ▅▅`.
fn render_core_strip(f: &mut Frame, area: Rect, theme: &Theme, cores: &[f32]) {
    const BARS: [char; 8] = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
    let n = cores.len();
    let w = area.width as usize;
    if n == 0 || w < n {
        return;
    }
    // Leave a one-cell gap between cores when there's room for it.
    let slot = w / n;
    let (cell, gap) = if slot >= 3 { (slot - 1, 1) } else { (slot, 0) };
    let spans: Vec<Span> = cores
        .iter()
        .flat_map(|&u| {
            let idx = ((u / 100.0) * 7.0).round().clamp(0.0, 7.0) as usize;
            [
                Span::styled(
                    BARS[idx].to_string().repeat(cell),
                    Style::default().fg(theme.usage(u as f64)),
                ),
                Span::raw(" ".repeat(gap)),
            ]
        })
        .collect();
    f.render_widget(Paragraph::new(Line::from(spans)), area);
}

/// Monitor-page CPU panel: header, big history graph, per-core meters.
pub fn render(f: &mut Frame, area: Rect, theme: &Theme, is_detailed: bool) {
    if area.height < 3 || area.width < 20 {
        return;
    }
    let snap = snapshot();
    let core_count = snap.cores.len();

    // Two cores per line when wide enough, otherwise one.
    let per_line = if area.width >= 36 { 2 } else { 1 };
    let mut core_rows = if is_detailed {
        core_count.div_ceil(per_line) as u16
    } else {
        0
    };
    // Cap the core block so the graph always keeps at least 2 rows.
    core_rows = core_rows.min(area.height.saturating_sub(4));

    // Compact mode: a one-row per-core heat strip under the graph.
    let strip = !is_detailed && area.height >= 6 && core_count > 0;
    let mut constraints = vec![Constraint::Length(1), Constraint::Min(2)];
    if core_rows > 0 {
        constraints.push(Constraint::Length(1));
        constraints.push(Constraint::Length(core_rows));
    } else if strip {
        constraints.push(Constraint::Length(1));
    }
    let chunks = Layout::vertical(constraints).split(area);

    let color = theme.usage(snap.usage as f64);
    // In compact mode the panel title already carries usage/temp/freq.
    let mut header = if is_detailed {
        vec![Span::styled(
            format!("{:>3.0}%  ", snap.usage),
            Style::default().fg(color),
        )]
    } else {
        Vec::new()
    };

    if snap.user_pct > 0.5 || snap.sys_pct > 0.5 || snap.iowait_pct > 0.5 {
        header.push(Span::styled(
            format!("usr {:.0}%", snap.user_pct),
            Style::default().fg(theme.secondary),
        ));
        header.push(Span::styled(
            format!("  sys {:.0}%", snap.sys_pct),
            Style::default().fg(theme.yellow),
        ));
        header.push(Span::styled(
            format!("  io {:.0}%", snap.iowait_pct),
            Style::default().fg(theme.red),
        ));
    }

    header.push(Span::styled(
        format!(
            "  load {:.2} {:.2} {:.2}",
            snap.load.0, snap.load.1, snap.load.2
        ),
        Style::default().fg(theme.dim),
    ));

    if snap.freq_mhz > 0 && is_detailed {
        header.push(Span::styled(
            format!("  {:.1}GHz", snap.freq_mhz as f64 / 1000.0),
            Style::default().fg(theme.text),
        ));
    }
    if let Some(t) = snap.max_temp().filter(|_| is_detailed) {
        header.push(Span::styled(
            format!("  {:.0}°C", t),
            Style::default().fg(theme.temp(t)),
        ));
    }
    if area.width >= 70 && is_detailed {
        header.push(Span::styled(
            format!("  {} threads", core_count),
            Style::default().fg(theme.dim),
        ));
    }
    f.render_widget(Paragraph::new(Line::from(header)), chunks[0]);

    use crate::widgets::block_graph::BlockGraph;

    let _points = chunks[1].width as usize; // blockgraph handles its own sub_w
    let hist_usage = HISTORY_USAGE.lock().unwrap().recent(1000);

    // We force braille style for the mirrored CPU graph by temporarily setting it,
    // or wait, blockgraph respects the global setting.
    // Let's assume the user has braille style selected, or block style works too.
    f.render_widget(
        BlockGraph::new(&hist_usage)
            .max(100.0)
            .mirrored(true)
            .colors(theme.accent, theme.yellow, theme.red),
        chunks[1],
    );
    if strip && core_rows == 0 {
        render_core_strip(f, chunks[2], theme, &snap.cores);
        return;
    }
    if core_rows == 0 {
        return;
    }
    let cols = Layout::horizontal(vec![Constraint::Ratio(1, per_line as u32); per_line])
        .spacing(2)
        .split(chunks[3]);
    for (i, &usage) in snap.cores.iter().enumerate() {
        let row = (i / per_line) as u16;
        if row >= core_rows {
            break;
        }
        let col = cols[i % per_line];
        let cell = Rect::new(col.x, col.y + row, col.width, 1);
        let c = theme.usage(usage as f64);
        let label = format!("c{:<2} ", i);

        let temp_str = if let Some(t) = snap.temps.get(i) {
            format!(" {:>3.0}°C", t)
        } else {
            String::new()
        };

        let pct = format!("{:>3.0}%", usage);
        let right_label = format!("{}{}", pct, temp_str);

        let right_label_chars = right_label.chars().count() as u16;
        let label_chars = label.chars().count() as u16;

        let core_chunks = Layout::horizontal([
            Constraint::Length(label_chars),
            Constraint::Min(0),
            Constraint::Length(right_label_chars),
        ])
        .split(cell);

        let bar_w = core_chunks[1].width as usize;

        // 1. Left Label
        f.render_widget(
            Paragraph::new(Span::styled(label, Style::default().fg(theme.dim))),
            core_chunks[0],
        );

        // 2. Bar
        f.render_widget(
            Paragraph::new(Span::styled(
                meter::bar(usage as f64 / 100.0, bar_w),
                Style::default().fg(c),
            )),
            core_chunks[1],
        );

        // 3. Right Label (pct + temp)
        let mut right_spans = vec![Span::styled(format!("{:>4}", pct), Style::default().fg(c))];
        if let Some(t) = snap.temps.get(i) {
            right_spans.push(Span::styled(
                format!(" {:>3.0}°C", t),
                Style::default().fg(theme.temp(*t)),
            ));
        }

        f.render_widget(
            Paragraph::new(Line::from(right_spans)).alignment(ratatui::layout::Alignment::Right),
            core_chunks[2],
        );
    }
}
