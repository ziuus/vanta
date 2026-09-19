use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::Frame;

use crate::app::{App, PanelId};
use crate::monitors::{cpu, disk, gpu, memory, network, processes, system_info};
use crate::screens::{panel, too_small};

const MIN: (u16, u16) = (80, 24);

/// btop-style: a metrics band on top, the full process table below.
pub fn render(f: &mut Frame, area: Rect, app: &App) {
    if area.width < MIN.0 || area.height < MIN.1 {
        too_small(f, area, &app.theme, MIN);
        return;
    }
    let theme = &app.theme;
    let focus = |p: PanelId| app.focused_panel == Some(p);
    let show_gpu = app.config.widgets.gpu;

    // The band grows with the terminal but the table always keeps the majority.
    let panel_constraint = |c: Constraint, id: &str| -> Constraint {
        let adj = *app.panel_states.dash_vertical.get(id).unwrap_or(&0);
        let apply = |v: u16| -> u16 {
            if adj < 0 {
                v.saturating_sub(adj.unsigned_abs())
            } else {
                v.saturating_add(adj as u16)
            }
        };
        match c {
            Constraint::Percentage(p) => Constraint::Percentage(apply(p)),
            Constraint::Length(l) => Constraint::Length(apply(l)),
            Constraint::Min(m) => Constraint::Min(apply(m)),
            Constraint::Max(m) => Constraint::Max(apply(m)),
            Constraint::Ratio(n, d) => Constraint::Ratio(n, d),
            Constraint::Fill(f) => Constraint::Fill(f),
        }
    };

    let adj_cpu = *app.panel_states.dash_vertical.get("cpu").unwrap_or(&0);
    let band_h = ((area.height * 2 / 5).clamp(14, 22) as i16 + adj_cpu).max(10) as u16;
    let [band, table] =
        Layout::vertical([Constraint::Length(band_h), Constraint::Min(8)]).areas(area);

    let r = app.panel_states.dash_ratios;
    let cols = Layout::horizontal([
        Constraint::Percentage(r[0]),
        Constraint::Percentage(r[1]),
        Constraint::Percentage(r[2]),
    ])
    .split(band);

    let inner = panel(f, cols[0], "cpu", theme, focus(PanelId::Cpu));
    cpu::render(f, inner, theme, true);

    let col1 = Layout::vertical([
        panel_constraint(Constraint::Percentage(50), "memory"),
        panel_constraint(Constraint::Percentage(50), "disk"),
    ])
    .split(cols[1]);
    let inner = panel(f, col1[0], "memory", theme, focus(PanelId::Memory));
    memory::render(f, inner, theme, true);
    let inner = panel(f, col1[1], "disk", theme, focus(PanelId::Disk));
    disk::render(f, inner, theme, true);

    let col2 = if show_gpu {
        Layout::vertical([
            panel_constraint(Constraint::Percentage(40), "network"),
            panel_constraint(Constraint::Percentage(40), "gpu"),
            panel_constraint(Constraint::Length(5), "system"),
        ])
        .split(cols[2])
    } else {
        Layout::vertical([
            panel_constraint(Constraint::Min(4), "network"),
            Constraint::Length(0),
            panel_constraint(Constraint::Length(5), "system"),
        ])
        .split(cols[2])
    };
    let inner = panel(f, col2[0], "network", theme, focus(PanelId::Network));
    network::render(f, inner, theme, true);
    if show_gpu {
        let inner = panel(f, col2[1], "gpu", theme, focus(PanelId::Gpu));
        gpu::render(f, inner, theme, true);
    }
    let inner = panel(f, col2[2], "system", theme, focus(PanelId::System));
    system_info::render(f, inner, theme, &app.summary);

    let ps = &app.panel_states;
    let title = format!(
        "processes · {}{}",
        processes::count(),
        if ps.process_tree_mode { " · tree" } else { "" }
    );
    let inner = panel(f, table, &title, theme, focus(PanelId::Processes));
    processes::render(
        f,
        inner,
        theme,
        ps.process_scroll_offset,
        ps.process_sort_field,
        ps.process_sort_asc,
        &ps.process_search,
        ps.process_search_active,
        ps.process_tree_mode,
        &ps.process_collapsed,
        ps.process_selected_pid,
        ps.process_compact_cmd,
    );
}
