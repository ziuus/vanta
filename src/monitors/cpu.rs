use std::fs;
use std::sync::{LazyLock, Mutex};

use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::monitors::history::History;
use crate::theme::Theme;
use crate::widgets::block_graph::BlockGraph;
use crate::widgets::meter;

#[derive(Clone, Default)]
pub struct CpuSnapshot {
    pub usage: f32,
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
static HISTORY: LazyLock<Mutex<History<240>>> = LazyLock::new(|| Mutex::new(History::new()));

pub fn snapshot() -> CpuSnapshot {
    SNAP.lock().unwrap().clone()
}

/// Called once per tick with a freshly refreshed sysinfo handle.
pub fn sample(sys: &sysinfo::System) {
    let la = sysinfo::System::load_average();
    let snap = CpuSnapshot {
        usage: sys.global_cpu_usage(),
        cores: sys.cpus().iter().map(|c| c.cpu_usage()).collect(),
        load: (la.one, la.five, la.fifteen),
        freq_mhz: sys.cpus().iter().map(|c| c.frequency()).max().unwrap_or(0),
        temps: read_core_temps(),
    };
    HISTORY.lock().unwrap().push(snap.usage as f64);
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

/// Monitor-page CPU panel: header, big history graph, per-core meters.
pub fn render(f: &mut Frame, area: Rect, theme: &Theme) {
    if area.height < 3 || area.width < 20 {
        return;
    }
    let snap = snapshot();
    let core_count = snap.cores.len();

    // Two cores per line when wide enough, otherwise one.
    let per_line = if area.width >= 36 { 2 } else { 1 };
    let core_rows = core_count.div_ceil(per_line) as u16;
    // Cap the core block so the graph always keeps at least 2 rows.
    let core_rows = core_rows.min(area.height.saturating_sub(4));

    let mut constraints = vec![Constraint::Length(1), Constraint::Min(2)];
    if core_rows > 0 {
        constraints.push(Constraint::Length(1));
        constraints.push(Constraint::Length(core_rows));
    }
    let chunks = Layout::vertical(constraints).split(area);

    let color = theme.usage(snap.usage as f64);
    let mut header = vec![
        Span::styled(
            format!("{:>3.0}%", snap.usage),
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!(
                "  load {:.2} {:.2} {:.2}",
                snap.load.0, snap.load.1, snap.load.2
            ),
            Style::default().fg(theme.dim),
        ),
    ];
    if snap.freq_mhz > 0 {
        header.push(Span::styled(
            format!("  {:.1}GHz", snap.freq_mhz as f64 / 1000.0),
            Style::default().fg(theme.text),
        ));
    }
    if let Some(t) = snap.max_temp() {
        header.push(Span::styled(
            format!("  {:.0}°C", t),
            Style::default().fg(theme.temp(t)),
        ));
    }
    if area.width >= 56 {
        header.push(Span::styled(
            format!("  {} threads", core_count),
            Style::default().fg(theme.dim),
        ));
    }
    f.render_widget(Paragraph::new(Line::from(header)), chunks[0]);

    let hist = HISTORY.lock().unwrap().recent(chunks[1].width as usize);
    f.render_widget(
        BlockGraph::new(&hist)
            .max(100.0)
            .colors(theme.accent, theme.yellow, theme.red),
        chunks[1],
    );

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
        let pct = format!("{:>4.0}%", usage);
        let bar_w = cell.width.saturating_sub((label.len() + pct.len()) as u16) as usize;
        f.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(label, Style::default().fg(theme.dim)),
                Span::styled(
                    meter::bar(usage as f64 / 100.0, bar_w),
                    Style::default().fg(c),
                ),
                Span::styled(pct, Style::default().fg(c).add_modifier(Modifier::BOLD)),
            ])),
            cell,
        );
    }
}
