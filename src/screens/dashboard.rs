use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::app::{App, PanelId};
use crate::monitors::{cpu, disk, memory, network, processes, system_info};
use crate::screens::{panel, too_small};
use crate::widgets::{calendar, clock, gauge, matrix, media, meter, music_viz, status};

const MIN: (u16, u16) = (96, 30);

/// Three columns: hardware · ambient · environment. Every panel is sized to
/// its content so nothing is left as dead space.
pub fn render(f: &mut Frame, area: Rect, app: &App) {
    if area.width < MIN.0 || area.height < MIN.1 {
        too_small(f, area, &app.theme, MIN);
        return;
    }
    let theme = &app.theme;
    let cfg = &app.config.widgets;
    let sum = &app.summary;
    let focus = |p: PanelId| app.focused_panel == Some(p);
    let term = (f.area().width, f.area().height);

    // Determine whether we need a custom-widgets row at the bottom.
    // Only allocate space when there's enough room to show both the main
    // dashboard and the custom row legibly. 37 rows = 30 main + 7 custom.
    let n_custom = app
        .config
        .custom_widgets
        .iter()
        .filter(|c| c.enabled)
        .count();
    let custom_row_h: u16 = if n_custom > 0 && area.height >= 37 {
        7
    } else {
        0
    };

    let [main_area, custom_area] =
        Layout::vertical([Constraint::Min(0), Constraint::Length(custom_row_h)]).areas(area);

    let cols = Layout::horizontal([
        Constraint::Ratio(1, 3),
        Constraint::Ratio(1, 3),
        Constraint::Ratio(1, 3),
    ])
    .split(main_area);

    // ── LEFT: hardware ─────────────────────────────────────────
    {
        let mounts = disk::mounts().len().clamp(1, 4) as u16;
        let rows = Layout::vertical([
            Constraint::Length(12),                                    // SYSTEM
            Constraint::Length(gauge::H as u16 + 2),                   // GAUGES
            Constraint::Min(8),                                        // CPU
            Constraint::Length(if cfg.disk { mounts + 2 } else { 0 }), // STORAGE
        ])
        .split(cols[0]);

        let inner = panel(f, rows[0], "system", theme, focus(PanelId::System));
        system_info::render_neofetch(f, inner, theme, sum, term);

        let inner = panel(f, rows[1], "gauges", theme, focus(PanelId::Gauges));
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

        if cfg.cpu {
            let inner = panel(f, rows[2], "cpu", theme, focus(PanelId::Cpu));
            cpu::render(f, inner, theme);
        } else {
            let inner = panel(f, rows[2], "matrix", theme, false);
            matrix::render(f, inner, theme);
        }

        if cfg.disk {
            let inner = panel(f, rows[3], "storage", theme, focus(PanelId::Storage));
            disk::render_storage(f, inner, theme);
        }
    }

    // ── CENTER: ambient ────────────────────────────────────────
    {
        let rows = Layout::vertical([
            Constraint::Length(if cfg.clock { 9 } else { 0 }), // CLOCK: 5 glyph + gap + date
            Constraint::Length(if cfg.media { 6 } else { 0 }), // MEDIA
            Constraint::Length(if cfg.music_viz { 9 } else { 0 }), // VISUALIZER
            Constraint::Min(6),                                // TOP PROCESSES
        ])
        .split(cols[1]);

        if cfg.clock {
            let inner = panel(f, rows[0], "clock", theme, focus(PanelId::Clock));
            clock::render(f, inner, theme, app.config.ui.clock_24h);
        }
        if cfg.media {
            let inner = panel(f, rows[1], "now playing", theme, focus(PanelId::Media));
            media::render(f, inner, theme);
        }
        if cfg.music_viz {
            let inner = panel(f, rows[2], "visualizer", theme, focus(PanelId::Visualizer));
            music_viz::render(f, inner, theme, app.frame);
        }
        let inner = panel(
            f,
            rows[3],
            "top processes",
            theme,
            focus(PanelId::Processes),
        );
        if cfg.processes {
            render_top_procs(f, inner, theme);
        } else {
            matrix::render(f, inner, theme);
        }
    }

    // ── RIGHT: environment ─────────────────────────────────────
    {
        let mem_h = if cfg.memory && main_area.height >= 42 {
            7
        } else {
            0
        };
        let rows = Layout::vertical([
            Constraint::Length(9),                                 // STATUS
            Constraint::Length(mem_h),                             // MEMORY (tall terminals)
            Constraint::Min(8),                                    // NETWORK
            Constraint::Length(if cfg.calendar { 11 } else { 0 }), // CALENDAR
        ])
        .split(cols[2]);

        let inner = panel(f, rows[0], "status", theme, focus(PanelId::Status));
        status::render(f, inner, theme);

        if mem_h > 0 {
            let inner = panel(f, rows[1], "memory", theme, focus(PanelId::Memory));
            memory::render(f, inner, theme);
        }

        if cfg.network {
            let inner = panel(f, rows[2], "network", theme, focus(PanelId::Network));
            network::render(f, inner, theme);
        } else {
            let inner = panel(f, rows[2], "matrix", theme, false);
            matrix::render(f, inner, theme);
        }

        if cfg.calendar {
            let inner = panel(f, rows[3], "calendar", theme, focus(PanelId::Calendar));
            calendar::render(f, inner, theme, app.panel_states.calendar_month_offset);
        }
    }

    // ── BOTTOM: custom widgets ─────────────────────────────────
    if n_custom > 0 && custom_area.height > 0 {
        render_custom_row(f, custom_area, app, n_custom);
    }
}

/// Lay out all enabled custom widgets in a single horizontal row.
/// Each widget gets an equal share of the width; they're always at least
/// `MIN_W` columns wide — if there isn't room we render as many as fit.
fn render_custom_row(f: &mut Frame, area: Rect, app: &App, n_custom: usize) {
    const MIN_W: u16 = 20;
    let max_fit = ((area.width / MIN_W) as usize).max(1);
    let n = n_custom.min(max_fit);
    if n == 0 {
        return;
    }

    // Equal-width columns.
    let constraints: Vec<Constraint> = (0..n).map(|_| Constraint::Ratio(1, n as u32)).collect();
    let cols = Layout::horizontal(constraints).split(area);

    // Iterate over enabled widgets (same order as in for_mode()).
    let mut slot = 0usize;
    for (cfg_idx, cfg) in app.config.custom_widgets.iter().enumerate() {
        if !cfg.enabled {
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
                Style::default().fg(theme.text).add_modifier(Modifier::BOLD),
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
