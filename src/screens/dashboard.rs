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
    // The media panel hosts the visualizer itself unless the layout also has a
    // standalone visualizer panel.
    let embed_viz = !layout
        .iter()
        .flatten()
        .any(|n| matches!(n.to_lowercase().as_str(), "visualizer" | "viz"));
    let ctx = SizeCtx {
        mounts,
        total_height: main_area.height,
        media_active: media::current_player().is_some() || music_viz::audio_active(),
        embed_viz,
    };
    if embed_viz && layout.iter().flatten().any(|n| is_media_name(n)) {
        // Keep cava alive so audio from MPRIS-less players can expand the panel.
        music_viz::ensure_running();
    }

    for (c, &col_area) in cols.iter().enumerate() {
        let Some(col_names) = layout.get(c) else {
            break;
        };
        let items: Vec<(String, Size)> = col_names
            .iter()
            .filter(|name| PanelId::from_name(name, &app.config).is_some())
            .map(|name| {
                let adj = PanelId::from_name(name, &app.config)
                    .and_then(|id| {
                        let key = format!("{:?}", id).to_lowercase();
                        app.panel_states.dash_vertical.get(&key).copied()
                    })
                    .unwrap_or(0);
                (name.clone(), panel_size(name, &ctx, adj))
            })
            .collect();
        if items.is_empty() {
            continue;
        }
        let (names, constraints) = solve_column(items, col_area.height);
        let rows = Layout::vertical(constraints).split(col_area);
        for (p, &row_area) in names.iter().zip(rows.iter()) {
            render_dashboard_panel(f, row_area, p, app, &sum, term, embed_viz);
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

fn is_media_name(name: &str) -> bool {
    matches!(
        name.to_lowercase().as_str(),
        "media" | "now_playing" | "now-playing"
    )
}

struct SizeCtx {
    mounts: u16,
    total_height: u16,
    media_active: bool,
    embed_viz: bool,
}

/// How a panel wants to be sized within its column (heights include borders).
#[derive(Clone, Copy, Debug, PartialEq)]
struct Size {
    /// Height it gets when there's room.
    pref: u16,
    /// Smallest height at which it still renders something useful.
    min: u16,
    /// Takes whatever space is left over (graphs, lists).
    flex: bool,
    /// Higher survives longer when the column is too short.
    priority: u8,
}

fn panel_size(name: &str, ctx: &SizeCtx, adjustment: i16) -> Size {
    let n = name.to_lowercase();
    let flex = is_flex_panel(&n) || matches!(n.as_str(), "upnext" | "up_next" | "up-next" | "next");
    let (pref, min, priority): (u16, u16, u8) = match n.as_str() {
        "clock" => (10, 5, 10),
        "cpu" => (8, 6, 9),
        "processes" | "procs" | "top_processes" | "top-processes" => (6, 5, 8),
        "media" | "now_playing" | "now-playing" => match (ctx.media_active, ctx.embed_viz) {
            (false, _) => (3, 3, 8),
            (true, true) => (13, 5, 8),
            (true, false) => (6, 4, 8),
        },
        "weather" => (10, 3, 7),
        "memory" | "mem" => (if ctx.total_height >= 38 { 7 } else { 6 }, 4, 7),
        "upnext" | "up_next" | "up-next" | "next" => (6, 4, 7),
        "timer" | "pomodoro" | "focus" => (10, 3, 7),
        "calendar" | "cal" => (12, 4, 6),
        "network" | "net" => (7, 5, 6),
        "system" => (12, 7, 5),
        "storage" | "disk" => (ctx.mounts + 2, 3, 4),
        "gpu" => (7, 5, 4),
        "gauges" | "gauge" => (gauge::H as u16 + 2, gauge::H as u16 + 2, 3),
        "status" => (8, 4, 3),
        "visualizer" | "viz" => (9, 5, 3),
        "video" | "donut" => (9, 6, 3),
        "matrix" => (6, 4, 2),
        "pinned_media" | "media_preview" | "media-preview" | "image" => (10, 6, 4),
        "news" => (8, 5, 4),
        "notes" | "writer" | "obsidian" | "files" => (10, 6, 5),
        "tasks" | "agenda" => (6, 4, 6),
        "filespace_path" => (3, 3, 5),
        "filespace_sidebar" => (10, 6, 5),
        "filespace_queue" => (8, 5, 5),
        "cryptopulse_mood" => (8, 6, 5),
        "cryptopulse_stats" => (7, 5, 5),
        "mediadeck_transport" => (4, 4, 5),
        "mediadeck_signal" | "mediadeck_now_playing" => (6, 5, 5),
        "portwatch_main" => (8, 6, 5),
        _ if flex => (6, 6, 5),
        _ => (7, 5, 5),
    };
    let pref = (pref as i16 + adjustment).max(3) as u16;
    Size {
        pref,
        min: min.min(pref),
        flex,
        priority,
    }
}

/// Fit a column's panels into `avail` rows. Drops the lowest-priority panels
/// until every survivor's minimum fits, then shrinks rigid panels toward their
/// minimum (lowest priority first). Flex panels, or the last panel when there
/// are none, absorb the leftover space.
fn solve_column(mut items: Vec<(String, Size)>, avail: u16) -> (Vec<String>, Vec<Constraint>) {
    while items.len() > 1 && items.iter().map(|(_, s)| s.min).sum::<u16>() > avail {
        // Lowest priority goes first; among equals, the one lowest in the column.
        let drop = items
            .iter()
            .enumerate()
            .min_by_key(|(i, (_, s))| (s.priority, usize::MAX - i))
            .map(|(i, _)| i)
            .unwrap_or(items.len() - 1);
        items.remove(drop);
    }

    let want = |items: &[(String, Size)]| -> u16 {
        items
            .iter()
            .map(|(_, s)| if s.flex { s.min } else { s.pref })
            .sum()
    };
    while want(&items) > avail {
        let over = want(&items) - avail;
        let Some((_, s)) = items
            .iter_mut()
            .filter(|(_, s)| !s.flex && s.pref > s.min)
            .min_by_key(|(_, s)| s.priority)
        else {
            break;
        };
        s.pref -= over.min(s.pref - s.min);
    }

    let has_flex = items.iter().any(|(_, s)| s.flex);
    let last = items.len() - 1;
    let constraints = items
        .iter()
        .enumerate()
        .map(|(i, (_, s))| match (s.flex, has_flex) {
            (true, _) => Constraint::Min(s.min),
            (false, false) if i == last => Constraint::Min(s.pref),
            _ => Constraint::Length(s.pref),
        })
        .collect();
    (items.into_iter().map(|(n, _)| n).collect(), constraints)
}

fn render_dashboard_panel(
    f: &mut Frame,
    area: Rect,
    name: &str,
    app: &mut App,
    sum: &crate::monitors::Summary,
    term: (u16, u16),
    embed_viz: bool,
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
            let snap = cpu::snapshot();
            let mut cpu_rt = format!("{:.0}%", sum.cpu_pct);
            if let Some(t) = snap.max_temp() {
                cpu_rt.push_str(&format!(" · {:.0}°", t));
            }
            if snap.freq_mhz > 0 && area.width >= 40 {
                cpu_rt.push_str(&format!(" · {:.1}GHz", snap.freq_mhz as f64 / 1000.0));
            }
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
            let note = crate::widgets::upnext::next_note();
            clock::render_with_note(
                f,
                inner,
                theme,
                app.config.ui.clock_24h,
                &app.config.ui.clock_font,
                &app.config.ui.clock_style,
                &[],
                note.as_deref(),
            );
        }
        "media" | "now_playing" | "now-playing" => {
            let player = media::current_player();
            let rt = player.clone();
            let inner = panel_full(
                f,
                area,
                "now playing",
                rt.as_deref(),
                Some("Space play/pause · n/p track · <> vol"),
                theme,
                focus(PanelId::Media),
            );
            let active = player.is_some() || music_viz::audio_active();
            if embed_viz && active && inner.height >= 6 {
                let [info, _, viz] = Layout::vertical([
                    Constraint::Length(3),
                    Constraint::Length(1),
                    Constraint::Min(1),
                ])
                .areas(inner);
                media::render(f, info, theme);
                music_viz::render(f, viz, theme, app.frame);
            } else {
                media::render(f, inner, theme);
            }
        }
        "upnext" | "up_next" | "up-next" | "next" => {
            let inner = panel(f, area, "up next", theme, focus(PanelId::UpNext));
            crate::widgets::upnext::render(f, inner, theme);
        }
        "timer" | "pomodoro" | "focus" => {
            let focused = focus(PanelId::Timer);
            let inner = panel(f, area, "focus timer", theme, focused);
            crate::widgets::pomodoro::render(f, inner, theme, &app.config.ui, focused);
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
            let snap = crate::monitors::weather::snapshot();
            let weather_rt = snap.ready.then_some(snap.location);
            let inner = panel_full(
                f,
                area,
                "weather",
                weather_rt.as_deref(),
                None,
                theme,
                focus(PanelId::Weather),
            );
            crate::widgets::weather::render(f, inner, theme);
        }
        "memory" | "mem" => {
            // The RAM/SWAP rows carry the numbers; the title stays clean.
            let inner = panel_full(f, area, "memory", None, None, theme, focus(PanelId::Memory));
            memory::render(f, inner, theme, false);
        }
        "network" | "net" => {
            // The body already shows both rates; only repeat them in the title
            // when the panel is too short to draw its body.
            let net_rt = (area.height < 5).then(|| {
                format!(
                    "↓{} ↑{}",
                    meter::fmt_kbps(sum.rx_kbps),
                    meter::fmt_kbps(sum.tx_kbps)
                )
            });
            let inner = panel_full(
                f,
                area,
                "network",
                net_rt.as_deref(),
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

            if !matched {
                let p = panel(f, area, custom_id, theme, false);
                f.render_widget(
                    Paragraph::new(format!(
                        "Widget '{}' not found or uninstalled.\nRun 'vanta menu' to install extensions.",
                        custom_id
                    ))
                    .style(Style::default().fg(theme.dim)),
                    p,
                );
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

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx(media_active: bool) -> SizeCtx {
        SizeCtx {
            mounts: 3,
            total_height: 32,
            media_active,
            embed_viz: true,
        }
    }

    fn col(names: &[&str], media_active: bool) -> Vec<(String, Size)> {
        names
            .iter()
            .map(|n| (n.to_string(), panel_size(n, &ctx(media_active), 0)))
            .collect()
    }

    fn heights(cs: &[Constraint]) -> Vec<u16> {
        cs.iter()
            .map(|c| match c {
                Constraint::Length(n) | Constraint::Min(n) => *n,
                _ => 0,
            })
            .collect()
    }

    #[test]
    fn roomy_column_keeps_everything_at_preferred_size() {
        let (names, cs) =
            solve_column(col(&["weather", "calendar", "upnext", "status"], false), 48);
        assert_eq!(names, ["weather", "calendar", "upnext", "status"]);
        assert_eq!(heights(&cs), [10, 12, 4, 8]);
        assert_eq!(cs[2], Constraint::Min(4));
    }

    #[test]
    fn short_column_shrinks_low_priority_rigid_panels_first() {
        // 10 + 12 + 4 + 8 = 34 > 30: status (prio 3) gives up 4 rows first.
        let (names, cs) =
            solve_column(col(&["weather", "calendar", "upnext", "status"], false), 30);
        assert_eq!(names.len(), 4);
        assert_eq!(heights(&cs), [10, 12, 4, 4]);
    }

    #[test]
    fn tiny_column_drops_lowest_priority_panels() {
        // Minimums: weather 3, calendar 4, upnext 4, status 4 = 15 > 12.
        let (names, _) = solve_column(col(&["weather", "calendar", "upnext", "status"], false), 12);
        assert!(!names.contains(&"status".to_string()));
        assert!(names.contains(&"weather".to_string()));
    }

    #[test]
    fn media_collapses_when_idle() {
        assert_eq!(panel_size("media", &ctx(false), 0).pref, 3);
        assert!(panel_size("media", &ctx(true), 0).pref > 3);
    }
}
