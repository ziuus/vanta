use std::fs;

use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Gauge, Paragraph};
use ratatui::Frame;

use crate::app;

fn usage_color(usage: f32, theme: &app::Theme) -> Color {
    if usage < 30.0 {
        theme.green
    } else if usage < 60.0 {
        theme.yellow
    } else {
        theme.red
    }
}

fn read_core_temps() -> Vec<f64> {
    if let Ok(dir) = fs::read_dir("/sys/devices/platform/coretemp.0/hwmon/") {
        for entry in dir.flatten() {
            let hwmon_path = entry.path().join("name");
            if let Ok(name) = fs::read_to_string(&hwmon_path) {
                if name.trim() == "coretemp" {
                    let mut temps: Vec<(usize, f64)> = Vec::new();
                    if let Ok(temp_dir) = fs::read_dir(&entry.path()) {
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

fn read_core_freqs() -> Vec<u64> {
    let mut freqs = Vec::new();
    for i in 0..16 {
        let path = format!("/sys/devices/system/cpu/cpu{}/cpufreq/scaling_cur_freq", i);
        if let Ok(s) = fs::read_to_string(&path) {
            if let Ok(khz) = s.trim().parse::<u64>() {
                freqs.push(khz / 1000);
            } else {
                break;
            }
        } else {
            break;
        }
    }
    freqs
}

fn read_package_temp() -> Option<f64> {
    for i in 0..16 {
        let path = format!("/sys/class/thermal/thermal_zone{}/temp", i);
        if let Ok(s) = fs::read_to_string(&path) {
            let type_path = format!("/sys/class/thermal/thermal_zone{}/type", i);
            let is_cpu = fs::read_to_string(&type_path)
                .ok()
                .map(|t| t.trim().to_lowercase())
                .is_some_and(|t| t.contains("cpu") || t.contains("x86") || t.contains("acpi"));
            if is_cpu {
                if let Ok(millideg) = s.trim().parse::<f64>() {
                    return Some(millideg / 1000.0);
                }
            }
        }
    }
    let core_temps = read_core_temps();
    core_temps.first().copied()
}

pub fn render(f: &mut Frame, area: Rect, theme: &app::Theme, demo: bool) {
    // ── Collect data (real or demo) ──
    let (cpu_usage, load_vals, core_count, freq_mhz, temp_c, core_temps, core_freqs, core_usage) =
        if demo {
            // Stable beautiful fake data for screenshots
            (
                42.5_f32,
                (1.2_f64, 0.8_f64, 0.5_f64),
                8_usize,
                2800_u64,
                Some(58.0_f64),
                vec![0.0, 48.0, 52.0, 56.0, 61.0, 55.0, 49.0, 47.0, 44.0],
                vec![2800, 3100, 2400, 2900, 1800, 2600, 3200, 2200],
                vec![23.0, 45.0, 67.0, 12.0, 34.0, 56.0, 78.0, 5.0],
            )
        } else {
            let mut system = sysinfo::System::new_all();
            system.refresh_cpu_all();
            let cores: Vec<_> = system.cpus().iter().map(|c| c.cpu_usage()).collect();
            let la = sysinfo::System::load_average();
            (
                system.global_cpu_usage(),
                (la.one, la.five, la.fifteen),
                system.cpus().len(),
                system.cpus().first().map(|c| c.frequency()).unwrap_or(0),
                read_package_temp(),
                read_core_temps(),
                read_core_freqs(),
                cores,
            )
        };

    // Layout: compact header | per-core rows
    let rows = (core_count + 1) / 2;
    let mut constraints = vec![Constraint::Length(1)]; // header
    for _ in 0..rows {
        constraints.push(Constraint::Length(1)); // per-core pairs
    }
    let chunks = Layout::vertical(constraints).split(area);

    // ── Temperature warning check ──
    let max_core_temp = core_temps
        .iter()
        .skip(1) // skip package placeholder (index 0)
        .copied()
        .fold(f64::NEG_INFINITY, f64::max);
    let temp_warning = max_core_temp.is_finite() && max_core_temp > 70.0;
    let temp_critical = max_core_temp.is_finite() && max_core_temp > 85.0;

    // ── Compact header ──
    let mut info = String::new();
    let mut color = usage_color(cpu_usage, theme);
    if temp_critical {
        color = theme.red;
    } else if temp_warning {
        color = theme.yellow;
    }
    info.push_str(&format!(" CPU  {}  {:.1}%", '█', cpu_usage));
    info.push_str(&format!(
        "  load {:.2} {:.2} {:.2}",
        load_vals.0, load_vals.1, load_vals.2
    ));
    info.push_str(&format!("  |  {}c", core_count));
    if let Some(t) = temp_c {
        info.push_str(&format!("  ·  {}°C", t as u16));
    }
    if temp_critical {
        info.push_str("  CRITICAL");
    } else if temp_warning {
        info.push_str("  HOT");
    }
    if freq_mhz > 0 {
        info.push_str(&format!("  ·  {} MHz", freq_mhz));
    }
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(&info, Style::default().fg(color)))),
        chunks[0],
    );

    // ── Per-core rows (2 per line, Gauge widgets) ──
    for (i, row_area) in chunks[1..].iter().enumerate() {
        let col_chunks =
            Layout::horizontal([Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)]).split(*row_area);

        for &(col_idx, gauge_area) in &[(0, col_chunks[0]), (1, col_chunks[1])] {
            let idx = i * 2 + col_idx;
            if idx >= core_usage.len() {
                // Fill remaining slot with empty space
                f.render_widget(Paragraph::new(Line::from(Span::styled("", Style::default()))), gauge_area);
                continue;
            }

            let usage = core_usage[idx];
            let c = usage_color(usage, theme);

            // Compact label: cN  XX.X%  ·  XX°C  ·  XXXXMHz
            let mut label = format!("c{:>2} {:>5.1}%", idx, usage);
            if idx < core_temps.len().saturating_sub(1) {
                let t = core_temps[idx + 1];
                label.push_str(&format!(" · {}°C", t as u16));
            }
            if idx < core_freqs.len() {
                let f = core_freqs[idx];
                label.push_str(&format!(" · {}MHz", f));
            }

            f.render_widget(
                Gauge::default()
                    .gauge_style(Style::default().fg(c).bg(theme.surface))
                    .percent(usage as u16)
                    .label(label),
                gauge_area,
            );
        }
    }
}