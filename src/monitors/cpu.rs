use std::fs;

use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Sparkline};
use ratatui::Frame;

const HIST_LEN: usize = 120;
static mut CPU_HISTORY: [u64; HIST_LEN] = [0; HIST_LEN];
static mut CPU_IDX: usize = 0;

use crate::app;

fn usage_color(usage: f32) -> Color {
    // btop-inspired gradient: green → yellow → orange → deep orange → red
    if usage < 40.0 {
        Color::Rgb(80, 200, 100)  // green
    } else if usage < 60.0 {
        Color::Rgb(200, 180, 50)  // yellow
    } else if usage < 75.0 {
        Color::Rgb(220, 140, 40)  // orange
    } else if usage < 90.0 {
        Color::Rgb(220, 100, 50)  // deep orange
    } else {
        Color::Rgb(220, 70, 60)   // soft red (only for critical)
    }
}

fn read_core_temps() -> Vec<f64> {
    if let Ok(dir) = fs::read_dir("/sys/devices/platform/coretemp.0/hwmon/") {
        for entry in dir.flatten() {
            let hwmon_path = entry.path().join("name");
            if let Ok(name) = fs::read_to_string(&hwmon_path) {
                if name.trim() == "coretemp" {
                    let mut temps: Vec<(usize, f64)> = Vec::new();
                    if let Ok(temp_dir) = fs::read_dir(entry.path()) {
                        for te in temp_dir.flatten() {
                            let fname = te.file_name().to_string_lossy().to_string();
                            if fname.starts_with("temp") && fname.ends_with("_input") {
                                let num_part: usize = fname
                                    .strip_prefix("temp")
                                    .and_then(|s| s.strip_suffix("_input"))
                                    .and_then(|s| s.parse().ok())
                                    .unwrap_or(0);
                                if let Ok(val) = fs::read_to_string(te.path()) {
                                    if let Ok(millideg) = val.trim().parse::<f64>() {
                                        temps.push((num_part, millideg / 1000.0));
                                    }
                                }
                            }
                        }
                    }
                    temps.sort_by_key(|(idx, _)| *idx);
                    return temps.into_iter().map(|(_, t)| t).collect();
                }
            }
        }
    }
    Vec::new()
}

pub fn render(f: &mut Frame, area: Rect, theme: &app::Theme) {
    // ── Collect data ──
    let sys = crate::app::SYS.lock().unwrap();
    // sys.refresh_cpu_all();
    let cores: Vec<_> = sys.cpus().iter().map(|c| c.cpu_usage()).collect();
    let la = sysinfo::System::load_average();
    let cpu_usage = sys.global_cpu_usage();
    unsafe {
        CPU_HISTORY[CPU_IDX] = cpu_usage as u64;
        CPU_IDX = (CPU_IDX + 1) % HIST_LEN;
    }
    let (load_vals, core_count, freq_mhz) = (
        (la.one, la.five, la.fifteen),
        sys.cpus().len(),
        sys.cpus().first().map(|c| c.frequency()).unwrap_or(0),
    );

    let core_temps = read_core_temps();
    let max_core_temp = core_temps
        .iter()
        .skip(1)
        .copied()
        .fold(f64::NEG_INFINITY, f64::max);
    let has_temp = max_core_temp.is_finite();

    // ── Layout ──
    let core_rows = core_count.div_ceil(2);
    let mut constraints: Vec<ratatui::layout::Constraint> = Vec::with_capacity(3 + core_rows);
    constraints.push(Constraint::Length(1)); // header line
    constraints.push(Constraint::Length(2)); // sparkline
    constraints.push(Constraint::Length(1)); // blank spacer
    for _ in 0..core_rows {
        constraints.push(Constraint::Length(1)); // per-core row
    }
    let chunks = Layout::vertical(constraints).split(area);

    // ── Header: CPU % · load · freq · temp ──
    let header_color = usage_color(cpu_usage);
    let mut parts = vec![
        format!("{:.0}%", cpu_usage),
        format!("{:.2}/{:.2}/{:.2}", load_vals.0, load_vals.1, load_vals.2),
    ];
    if freq_mhz > 0 {
        parts.push(format!("{:.1}GHz", freq_mhz as f64 / 1000.0));
    }
    if has_temp {
        parts.push(format!("{}°C", max_core_temp as u64));
    }
    parts.push(format!("{}c", core_count));
    let header = parts.join(" · ");

    f.render_widget(
        Paragraph::new(Line::from(Span::styled(&header, Style::default().fg(header_color)))),
        chunks[0],
    );

    // ── Sparkline ──
    let hist: Vec<u64> = unsafe {
        let max_w = chunks[1].width.min(HIST_LEN as u16) as usize;
        (0..max_w)
            .map(|i| {
                let idx = (CPU_IDX + HIST_LEN - 1 - i) % HIST_LEN;
                CPU_HISTORY[idx]
            })
            .rev()
            .collect()
    };
    f.render_widget(
        Sparkline::default()
            .data(&hist)
            .max(100)
            .style(Style::default().fg(header_color)),
        chunks[1],
    );

    // ── Per-core rows (2 per line, compact gauges) ──
    for (i, row_area) in chunks[3..].iter().enumerate() {
        let col_chunks =
            Layout::horizontal([Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)]).split(*row_area);

        for &(col_idx, gauge_area) in &[(0, col_chunks[0]), (1, col_chunks[1])] {
            let idx = i * 2 + col_idx;
            if idx >= cores.len() {
                continue;
            }

            let usage = cores[idx];
            let c = usage_color(usage);
            
            // Render as: c0 ....... 50%
            let label_w = 3; // "c0 "
            let pct_w = 4; // " 50%"
            let dots_w = gauge_area.width.saturating_sub((label_w + pct_w) as u16) as usize;
            
            // Create the dotted line, dimmed
            let dots = if dots_w > 0 {
                "·".repeat(dots_w)
            } else {
                String::new()
            };

            let line = Line::from(vec![
                Span::styled(format!("c{:<2}", idx), Style::default().fg(Color::White)),
                Span::styled(dots, Style::default().fg(theme.dim)),
                Span::styled(format!("{:>3.0}%", usage), Style::default().fg(c)),
            ]);

            f.render_widget(Paragraph::new(line), gauge_area);
        }
    }
}