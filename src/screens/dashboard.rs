use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::app::{App, PanelId};
use crate::monitors::{cpu, disk, gpu, memory, network, processes, system_info};
use crate::screens::{panel, panel_full, too_small};
use crate::widgets::{calendar, clock, gauge, matrix, media, meter, music_viz, status};

const MIN: (u16, u16) = (80, 24);

/// Data-driven dashboard layout: reads `dashboard.layout` from config,
/// supporting user-defined column assignments, panel reordering, and custom widgets.
pub fn render(f: &mut Frame, area: Rect, app: &mut App) {
    let layout = app.config.dashboard.layout.clone();
    render_layout(f, area, app, &layout);
}

pub fn render_layout(f: &mut Frame, area: Rect, app: &mut App, layout: &[Vec<String>]) {
    if area.width < MIN.0 || area.height < MIN.1 {
        too_small(f, area, &app.theme, MIN);
        return;
    }
    let sum = app.summary.clone();
    let term = (f.area().width, f.area().height);

    let assigned_custom_ids: Vec<String> =
        layout.iter().flat_map(|col| col.iter().cloned()).collect();

    let is_main_dashboard = layout == app.config.dashboard.layout.as_slice();
    let unassigned_custom_count = if is_main_dashboard {
        app.config
            .custom_widgets
            .iter()
            .filter(|c| {
                c.enabled
                    && !assigned_custom_ids
                        .iter()
                        .any(|id| id.eq_ignore_ascii_case(&c.id))
            })
            .count()
    } else {
        0
    };

    let custom_row_h: u16 = if unassigned_custom_count > 0 && area.height >= 37 {
        7
    } else {
        0
    };

    let [main_area, custom_area] =
        Layout::vertical([Constraint::Min(0), Constraint::Length(custom_row_h)]).areas(area);

    let n_cols = layout.len().clamp(1, 4);
    let col_constraints: Vec<Constraint> =
        if is_main_dashboard && n_cols == 3 && app.panel_states.dash_ratios.len() == 3 {
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

    let columns_active: Vec<Vec<String>> = layout
        .iter()
        .map(|col_names| {
            col_names
                .iter()
                .filter(|name| PanelId::from_name(name, &app.config).is_some())
                .cloned()
                .collect()
        })
        .collect();

    for (c, &col_area) in cols.iter().enumerate() {
        if c >= columns_active.len() {
            break;
        }
        let active_panels = &columns_active[c];
        if active_panels.is_empty() {
            continue;
        }

        let has_flex = active_panels.iter().any(|p| is_flex_panel(p));
        let num_panels = active_panels.len();

        let mut flex_adj_total = 0;
        for p in active_panels.iter() {
            if is_flex_panel(p) {
                if let Some(id) = crate::app::PanelId::from_name(p, &app.config) {
                    let key = format!("{:?}", id).to_lowercase();
                    flex_adj_total += app
                        .panel_states
                        .dash_vertical
                        .get(&key)
                        .copied()
                        .unwrap_or(0);
                }
            }
        }

        // Distribute -flex_adj_total to rigid components
        let mut flex_remainder = -flex_adj_total;

        let constraints: Vec<Constraint> = active_panels
            .iter()
            .enumerate()
            .map(|(i, p)| {
                let mut adj = 0;
                if let Some(id) = crate::app::PanelId::from_name(p, &app.config) {
                    let key = format!("{:?}", id).to_lowercase();
                    adj = app
                        .panel_states
                        .dash_vertical
                        .get(&key)
                        .copied()
                        .unwrap_or(0);
                }
                if !is_flex_panel(p) && flex_remainder != 0 {
                    // Try to give this rigid component 2 or -2 units of the remainder, or whatever is left
                    let chunk = if flex_remainder > 0 {
                        flex_remainder.min(2)
                    } else {
                        flex_remainder.max(-2)
                    };
                    adj += chunk;
                    flex_remainder -= chunk;
                }
                panel_constraint(
                    p,
                    i == num_panels - 1,
                    has_flex,
                    mounts,
                    main_area.height,
                    adj,
                )
            })
            .collect();

        let rows = Layout::vertical(constraints).split(col_area);
        for (p, &row_area) in active_panels.iter().zip(rows.iter()) {
            render_dashboard_panel(f, row_area, p, app, &sum, term);
        }
    }

    if unassigned_custom_count > 0 && custom_area.height > 0 {
        render_custom_row(f, custom_area, app, unassigned_custom_count, layout);
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
            | "tasks"
            | "agenda"
            | "notes"
            | "writer"
            | "obsidian"
            | "news"
            | "pinned_media"
            | "media_preview"
            | "media-preview"
            | "image"
            | "files"
            | "filespace_browser"
            | "filespace_preview"
            | "crypto_coin"
            | "cryptopulse_overview"
            | "cryptopulse_movers"
            | "cryptopulse_watchlist"
            | "cryptopulse_heatmap"
            | "mediadeck_visualizer"
            | "mediadeck_players"
            | "sentinel"
            | "proctrace_main"
            | "netscope_main"
            | "iowatch_main"
            | "servicewatch_main"
    )
}

fn panel_constraint(
    name: &str,
    is_last: bool,
    has_flex_in_col: bool,
    mounts: u16,
    total_height: u16,
    adjustment: i16,
) -> Constraint {
    let apply = |base: u16| -> u16 { (base as i16 + adjustment).max(3) as u16 };

    if is_flex_panel(name) {
        if name.eq_ignore_ascii_case("cpu") {
            Constraint::Min(apply(8))
        } else {
            Constraint::Min(apply(6))
        }
    } else {
        let h = match name.to_lowercase().as_str() {
            "filespace_path" => 3,
            "filespace_sidebar" => 10,
            "filespace_queue" => 8,
            "cryptopulse_mood" => 8,
            "cryptopulse_stats" => 7,
            "mediadeck_transport" => 4,
            "mediadeck_signal" => 6,
            "mediadeck_now_playing" => 6,
            "portwatch_main" => 8,
            "system" => 12,
            "gauges" | "gauge" => gauge::H as u16 + 2,
            "storage" | "disk" => mounts + 2,
            "clock" => 9,
            "media" | "now_playing" | "now-playing" => 6,
            "visualizer" | "viz" => 9,
            "status" => 8,
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
            "network" | "net" => 7,
            "video" | "donut" => 9,
            "pinned_media" | "media_preview" | "media-preview" | "image" => 10,
            "news" => 8,
            "notes" | "writer" | "obsidian" => 10,
            "files" => 10,
            _ => 7,
        };
        if is_last && !has_flex_in_col {
            Constraint::Min(apply(h))
        } else {
            Constraint::Length(apply(h))
        }
    }
}

fn render_dashboard_panel(
    f: &mut Frame,
    area: Rect,
    name: &str,
    app: &mut App,
    sum: &crate::monitors::Summary,
    term: (u16, u16),
) {
    let theme_val = app.theme.clone();
    let theme = &theme_val;
    let focused_panel = app.focused_panel;
    let focus = |p: PanelId| focused_panel == Some(p);

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
            cpu::render(f, inner, theme, false);
        }
        "storage" => {
            let inner = panel(f, area, "storage", theme, focus(PanelId::Storage));
            disk::render_storage(f, inner, theme);
        }
        "disk" => {
            let inner = panel(f, area, "disk", theme, focus(PanelId::Disk));
            disk::render(f, inner, theme, false);
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
            let inner = panel_full(
                f,
                area,
                "now playing",
                None,
                Some("Space play/pause · n/p track · <> vol"),
                theme,
                focus(PanelId::Media),
            );
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
            memory::render(f, inner, theme, false);
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
            network::render(f, inner, theme, false);
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
            gpu::render(f, inner, theme, false);
        }
        "agenda" => {
            let snap = crate::monitors::agenda::snapshot();
            let count = snap.events.len();
            let agenda_rt = if count == 0 {
                " no events ".to_string()
            } else {
                format!(" {} upcoming ", count)
            };
            let hint = if app.panel_states.agenda_input_active {
                "Enter submit · Esc cancel"
            } else {
                "a add · d del · e edit · ↑↓ select"
            };
            let inner = panel_full(
                f,
                area,
                "agenda",
                Some(&agenda_rt),
                Some(hint),
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
            let hint = if app.panel_states.task_input_active {
                "Enter submit · Esc cancel"
            } else {
                "a add · Space toggle · d del · e edit"
            };
            let inner = panel_full(
                f,
                area,
                "tasks",
                Some(&tasks_rt),
                Some(hint),
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
        "news" => {
            let snap = crate::monitors::news::snapshot();
            let source = if snap.channel_title.is_empty() {
                "".to_string()
            } else {
                format!(" {} ", snap.channel_title)
            };
            let inner = panel_full(
                f,
                area,
                "news",
                (!source.is_empty()).then_some(&source),
                None,
                theme,
                focus(PanelId::News),
            );
            crate::widgets::news::render(f, inner, theme);
        }
        "pinned_media" | "media_preview" | "media-preview" | "image" => {
            let hint = if app.panel_states.pinned_media_input_active {
                format!("media path: {}_", app.panel_states.pinned_media_input)
            } else {
                "media preview".to_string()
            };
            let inner = panel(f, area, &hint, theme, focus(PanelId::PinnedMedia));
            crate::widgets::pinned_media::render(
                f,
                inner,
                theme,
                &app.config.ui.pinned_media_path,
                app.frame,
            );
        }
        "video" | "donut" => {
            let inner = panel(f, area, "video", theme, focus(PanelId::Video));
            crate::widgets::video::render_with_motion(
                f,
                inner,
                theme,
                app.frame,
                app.config.ui.motion_enabled,
                app.config.ui.motion_speed,
                &app.config.ui.motion_mode,
            );
        }
        "notes" | "writer" | "obsidian" => {
            crate::screens::workspace::render_notes(f, area, app);
        }
        "files" => {
            let inner = panel(f, area, "yazi (file manager)", theme, focus(PanelId::Files));
            let files_focused = focus(PanelId::Files);
            crate::widgets::files::render(
                f,
                inner,
                theme,
                files_focused,
                &mut app.panel_states.files_selected,
                &mut app.panel_states.files_scroll,
            );
        }
        custom_id => {
            let mut matched = false;
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
                matched = true;
            }

            if !matched {
                for ext in &app.ext_manager.extensions {
                    for mut comp in ext.components() {
                        if comp.id().eq_ignore_ascii_case(custom_id)
                            || ext.metadata().id.eq_ignore_ascii_case(custom_id)
                        {
                            comp.render(f, area, theme);
                            matched = true;
                            break;
                        }
                    }
                    if matched {
                        break;
                    }
                }
            }
        }
    }
}

fn render_custom_row(
    f: &mut Frame,
    area: Rect,
    app: &App,
    unassigned_count: usize,
    layout: &[Vec<String>],
) {
    const MIN_W: u16 = 20;
    let max_fit = ((area.width / MIN_W) as usize).max(1);
    let n = unassigned_count.min(max_fit);
    if n == 0 {
        return;
    }

    let constraints: Vec<Constraint> = (0..n).map(|_| Constraint::Ratio(1, n as u32)).collect();
    let cols = Layout::horizontal(constraints).split(area);

    let assigned_custom_ids: Vec<String> =
        layout.iter().flat_map(|col| col.iter().cloned()).collect();

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
