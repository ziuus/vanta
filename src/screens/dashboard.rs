use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::app::{App, PanelId};
use crate::monitors::{cpu, disk, gpu, memory, network, processes, system_info};
use crate::screens::{panel, panel_full, too_small};
use crate::widgets::{calendar, clock, gauge, matrix, media, meter, music_viz, status};

const MIN: (u16, u16) = (96, 30);

/// Data-driven dashboard layout: reads `dashboard.layout` from config,
/// supporting user-defined column assignments, panel reordering, and custom widgets.
pub fn render(f: &mut Frame, area: Rect, app: &App) {
    if area.width < MIN.0 || area.height < MIN.1 {
        too_small(f, area, &app.theme, MIN);
        return;
    }
    let sum = &app.summary;
    let term = (f.area().width, f.area().height);

    let assigned_custom_ids: Vec<String> = app
        .config
        .dashboard
        .layout
        .iter()
        .flat_map(|col| col.iter().cloned())
        .collect();

    let unassigned_custom_count = app
        .config
        .custom_widgets
        .iter()
        .filter(|c| {
            c.enabled
                && !assigned_custom_ids
                    .iter()
                    .any(|id| id.eq_ignore_ascii_case(&c.id))
        })
        .count();

    let custom_row_h: u16 = if unassigned_custom_count > 0 && area.height >= 37 {
        7
    } else {
        0
    };

    let [main_area, custom_area] =
        Layout::vertical([Constraint::Min(0), Constraint::Length(custom_row_h)]).areas(area);

    let n_cols = app.config.dashboard.layout.len().clamp(1, 4);
    let col_constraints: Vec<Constraint> = if n_cols == 3 && app.panel_states.dash_ratios.len() == 3
    {
        vec![
            Constraint::Percentage(app.panel_states.dash_ratios[0]),
            Constraint::Percentage(app.panel_states.dash_ratios[1]),
            Constraint::Percentage(app.panel_states.dash_ratios[2]),
        ]
    } else {
        (0..n_cols)
            .map(|_| Constraint::Ratio(1, n_cols as u32))
            .collect()
    };
    let cols = Layout::horizontal(col_constraints).split(main_area);

    let mounts = disk::mounts().len().clamp(1, 4) as u16;

    for (c, &col_area) in cols.iter().enumerate() {
        if c >= app.config.dashboard.layout.len() {
            break;
        }
        let col_names = &app.config.dashboard.layout[c];
        let active_panels: Vec<&str> = col_names
            .iter()
            .map(|s| s.as_str())
            .filter(|name| PanelId::from_name(name, &app.config).is_some())
            .collect();

        if active_panels.is_empty() {
            continue;
        }

        let has_flex = active_panels.iter().any(|p| is_flex_panel(p));
        let num_panels = active_panels.len();
        let constraints: Vec<Constraint> = active_panels
            .iter()
            .enumerate()
            .map(|(i, &p)| {
                panel_constraint(p, i == num_panels - 1, has_flex, mounts, main_area.height)
            })
            .collect();

        let rows = Layout::vertical(constraints).split(col_area);
        for (&p, &row_area) in active_panels.iter().zip(rows.iter()) {
            render_dashboard_panel(f, row_area, p, app, sum, term);
        }
    }

    if unassigned_custom_count > 0 && custom_area.height > 0 {
        render_custom_row(f, custom_area, app, unassigned_custom_count);
    }
}

fn is_flex_panel(name: &str) -> bool {
    matches!(
        name.to_lowercase().as_str(),
        "cpu"
            | "processes"
            | "procs"
            | "top_processes"
            | "top-processes"
            | "network"
            | "net"
            | "matrix"
    )
}

fn panel_constraint(
    name: &str,
    is_last: bool,
    has_flex_in_col: bool,
    mounts: u16,
    total_height: u16,
) -> Constraint {
    if is_flex_panel(name) {
        if name.eq_ignore_ascii_case("cpu") {
            Constraint::Min(8)
        } else {
            Constraint::Min(6)
        }
    } else {
        let h = match name.to_lowercase().as_str() {
            "system" => 12,
            "gauges" | "gauge" => gauge::H as u16 + 2,
            "storage" | "disk" => mounts + 2,
            "clock" => 9,
            "media" | "now_playing" | "now-playing" => 6,
            "visualizer" | "viz" => 9,
            "status" => 9,
            "weather" => 9,
            "calendar" | "cal" => 12,
            "memory" | "mem" => {
                if total_height >= 38 {
                    7
                } else {
                    6
                }
            }
            "gpu" => 7,
            _ => 7,
        };
        if is_last && !has_flex_in_col {
            Constraint::Min(h)
        } else {
            Constraint::Length(h)
        }
    }
}

fn render_dashboard_panel(
    f: &mut Frame,
    area: Rect,
    name: &str,
    app: &App,
    sum: &crate::monitors::Summary,
    term: (u16, u16),
) {
    let theme = &app.theme;
    let focus = |p: PanelId| app.focused_panel == Some(p);

    match name.to_lowercase().as_str() {
        "system" => {
            let inner = panel(f, area, "system", theme, focus(PanelId::System));
            system_info::render_neofetch(f, inner, theme, sum, term);
        }
        "gauges" | "gauge" => {
            let inner = panel(f, area, "gauges", theme, focus(PanelId::Gauges));
            let mut metrics = vec![
                (
                    "cpu",
                    sum.cpu_pct as f64,
                    format!("{:.0}%", sum.cpu_pct),
                    theme.usage(sum.cpu_pct as f64),
                ),
                (
                    "mem",
                    sum.mem_pct,
                    format!("{:.0}%", sum.mem_pct),
                    theme.usage(sum.mem_pct),
                ),
            ];
            match (sum.gpu_pct, sum.battery) {
                (Some(g), _) => metrics.push(("gpu", g, format!("{:.0}%", g), theme.usage(g))),
                (None, Some((b, charging))) => {
                    let col = if charging || b > 40 {
                        theme.green
                    } else if b > 20 {
                        theme.yellow
                    } else {
                        theme.red
                    };
                    metrics.push((
                        "bat",
                        b as f64,
                        format!("{}%{}", b, if charging { "⚡" } else { "" }),
                        col,
                    ));
                }
                (None, None) => {
                    if let Some(d) = sum.disk_pct {
                        metrics.push(("disk", d, format!("{:.0}%", d), theme.usage(d)));
                    }
                }
            }
            gauge::render(f, inner, theme, &metrics);
        }
        "cpu" => {
            let cpu_rt = format!(" {:.1}% ", sum.cpu_pct);
            let inner = panel_full(
                f,
                area,
                "cpu",
                Some(&cpu_rt),
                None,
                theme,
                focus(PanelId::Cpu),
            );
            cpu::render(f, inner, theme);
        }
        "storage" => {
            let inner = panel(f, area, "storage", theme, focus(PanelId::Storage));
            disk::render_storage(f, inner, theme);
        }
        "disk" => {
            let inner = panel(f, area, "disk", theme, focus(PanelId::Disk));
            disk::render(f, inner, theme);
        }
        "clock" => {
            let inner = panel(f, area, "clock", theme, focus(PanelId::Clock));
            clock::render(
                f,
                inner,
                theme,
                app.config.ui.clock_24h,
                &app.config.ui.clock_font,
                &app.config.ui.clock_style,
                &[],
            );
        }
        "media" | "now_playing" | "now-playing" => {
            let inner = panel(f, area, "now playing", theme, focus(PanelId::Media));
            media::render(f, inner, theme);
        }
        "visualizer" | "viz" => {
            let inner = panel(f, area, "visualizer", theme, focus(PanelId::Visualizer));
            music_viz::render(f, inner, theme, app.frame);
        }
        "processes" | "procs" | "top_processes" | "top-processes" => {
            let inner = panel_full(
                f,
                area,
                "top processes",
                None,
                Some("↑ ↓ scroll • k kill"),
                theme,
                focus(PanelId::Processes),
            );
            render_top_procs(f, inner, theme);
        }
        "status" => {
            let inner = panel_full(
                f,
                area,
                "status",
                None,
                Some("e to manage"),
                theme,
                focus(PanelId::Status),
            );
            status::render(
                f,
                inner,
                theme,
                focus(PanelId::Status),
                app.panel_states.status_selected,
            );
        }
        "weather" => {
            let weather_rt = if crate::monitors::weather::snapshot().ready {
                format!(" {} ", crate::monitors::weather::snapshot().location)
            } else {
                " offline ".to_string()
            };
            let inner = panel_full(
                f,
                area,
                "weather",
                Some(&weather_rt),
                None,
                theme,
                focus(PanelId::Weather),
            );
            crate::widgets::weather::render(f, inner, theme);
        }
        "memory" | "mem" => {
            let mem_rt = format!(" {:.1}% ", sum.mem_pct);
            let inner = panel_full(
                f,
                area,
                "memory",
                Some(&mem_rt),
                None,
                theme,
                focus(PanelId::Memory),
            );
            memory::render(f, inner, theme);
        }
        "network" | "net" => {
            let net_rt = format!(" ↓{:.0} ↑{:.0} kb/s ", sum.rx_kbps, sum.tx_kbps);
            let inner = panel_full(
                f,
                area,
                "network",
                Some(&net_rt),
                None,
                theme,
                focus(PanelId::Network),
            );
            network::render(f, inner, theme);
        }
        "calendar" | "cal" => {
            let cal_hint = if app.panel_states.calendar_month_offset == 0 {
                "← → month"
            } else {
                "Home reset · ← →"
            };
            let inner = panel_full(
                f,
                area,
                "calendar",
                None,
                Some(cal_hint),
                theme,
                focus(PanelId::Calendar),
            );
            calendar::render(f, inner, theme, app.panel_states.calendar_month_offset);
        }
        "matrix" => {
            let inner = panel(f, area, "matrix", theme, false);
            matrix::render(f, inner, theme);
        }
        "gpu" => {
            let inner = panel(f, area, "gpu", theme, focus(PanelId::Gpu));
            gpu::render(f, inner, theme);
        }
        "agenda" => {
            let snap = crate::monitors::agenda::snapshot();
            let count = snap.events.len();
            let agenda_rt = if count == 0 {
                " no events ".to_string()
            } else {
                format!(" {} upcoming ", count)
            };
            let inner = panel_full(
                f,
                area,
                "agenda",
                Some(&agenda_rt),
                None,
                theme,
                focus(PanelId::Agenda),
            );
            crate::widgets::agenda::render(
                f,
                inner,
                theme,
                focus(PanelId::Agenda),
                app.panel_states.agenda_selected,
                app.panel_states.agenda_input_active,
                &app.panel_states.agenda_input,
            );
        }
        "tasks" => {
            let snap = crate::monitors::tasks::snapshot();
            let open = snap.tasks.iter().filter(|t| !t.completed).count();
            let tasks_rt = format!(" {} open ", open);
            let inner = panel_full(
                f,
                area,
                "tasks",
                Some(&tasks_rt),
                None,
                theme,
                focus(PanelId::Tasks),
            );
            crate::widgets::tasks::render(
                f,
                inner,
                theme,
                focus(PanelId::Tasks),
                app.panel_states.tasks_selected,
                app.panel_states.task_input_active,
                &app.panel_states.task_input,
            );
        }
        custom_id => {
            if let Some((cfg_idx, _)) = app
                .config
                .custom_widgets
                .iter()
                .enumerate()
                .find(|(_, cw)| cw.id.eq_ignore_ascii_case(custom_id))
            {
                let focused = app.focused_panel == Some(PanelId::Custom(cfg_idx));
                app.custom_widgets
                    .render_widget(f, area, cfg_idx, focused, theme);
            }
        }
    }
}

fn render_custom_row(f: &mut Frame, area: Rect, app: &App, unassigned_count: usize) {
    const MIN_W: u16 = 20;
    let max_fit = ((area.width / MIN_W) as usize).max(1);
    let n = unassigned_count.min(max_fit);
    if n == 0 {
        return;
    }

    let constraints: Vec<Constraint> = (0..n).map(|_| Constraint::Ratio(1, n as u32)).collect();
    let cols = Layout::horizontal(constraints).split(area);

    let assigned_custom_ids: Vec<String> = app
        .config
        .dashboard
        .layout
        .iter()
        .flat_map(|col| col.iter().cloned())
        .collect();

    let mut slot = 0usize;
    for (cfg_idx, cfg) in app.config.custom_widgets.iter().enumerate() {
        if !cfg.enabled
            || assigned_custom_ids
                .iter()
                .any(|id| id.eq_ignore_ascii_case(&cfg.id))
        {
            continue;
        }
        if slot >= n {
            break;
        }
        let focused = app.focused_panel == Some(PanelId::Custom(cfg_idx));
        app.custom_widgets
            .render_widget(f, cols[slot], cfg_idx, focused, &app.theme);
        slot += 1;
    }
}

fn render_top_procs(f: &mut Frame, area: Rect, theme: &crate::theme::Theme) {
    if area.height < 2 || area.width < 24 {
        return;
    }
    let max_rows = (area.height as usize).saturating_sub(1);
    let procs = processes::top_by_cpu(max_rows);

    let w = area.width as usize;
    let bar_w = if w >= 56 { 10 } else { 0 };
    let name_w = w.saturating_sub(7 + 7 + 7 + bar_w + 3).max(6);
    let hs = Style::default().fg(theme.dim);

    let mut lines = vec![Line::from(vec![
        Span::styled(format!("{:>6} ", "pid"), hs),
        Span::styled(format!("{:<w$} ", "name", w = name_w), hs),
        Span::styled(format!("{:>6} ", "cpu%"), hs),
        Span::styled(format!("{:>6}", "mem"), hs),
        Span::styled(" ".repeat(bar_w + 1), hs),
    ])];
    for p in procs {
        let cpu_col = if p.cpu_pct >= 50.0 {
            theme.red
        } else if p.cpu_pct >= 10.0 {
            theme.yellow
        } else {
            theme.text
        };
        let mut spans = vec![
            Span::styled(format!("{:>6} ", p.pid), Style::default().fg(theme.dim)),
            Span::styled(
                format!("{:<w$} ", meter::ellipsize(&p.name, name_w), w = name_w),
                Style::default().fg(theme.text),
            ),
            Span::styled(
                format!("{:>5.1}% ", p.cpu_pct),
                Style::default().fg(cpu_col),
            ),
            Span::styled(
                format!("{:>6}", meter::fmt_bytes(p.mem_kb * 1024)),
                Style::default().fg(theme.secondary),
            ),
        ];
        if bar_w > 0 {
            spans.push(Span::styled(
                format!(" {}", meter::bar((p.cpu_pct / 100.0).min(1.0), bar_w)),
                Style::default().fg(cpu_col),
            ));
        }
        lines.push(Line::from(spans));
    }
    f.render_widget(Paragraph::new(lines), area);
}
