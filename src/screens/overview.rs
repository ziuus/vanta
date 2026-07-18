use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::Span;
use ratatui::Frame;

use crate::app::{self, PanelId, PanelStates};
use crate::config::Config;
use crate::monitors::{cpu, disk, gpu, memory, network, processes, system_info};
use crate::widgets::{calendar, clock, cmatrix, media, music_viz};

/// Render a btop-style colored section label as the first line of a panel.
/// Returns the remaining area below the label for the widget content.
use ratatui::widgets::{Block, Borders, BorderType};

fn section_header(f: &mut Frame, area: Rect, label: &str, theme: &app::Theme, focused: bool) -> Rect {
    let title_color = if focused { theme.focus } else { theme.accent };
    let border_color = if focused { theme.focus } else { theme.dim };
    
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color))
        .title(Span::styled(
            format!(" {} ", label),
            Style::default().fg(title_color).add_modifier(Modifier::BOLD),
        ));
        
    let inner = block.inner(area);
    f.render_widget(block, area);
    inner
}

pub fn render(
    f: &mut Frame,
    area: Rect,
    theme: &app::Theme,
    config: &Config,
    tick: u64,
    focused: Option<PanelId>,
    states: &PanelStates,
) {
    // Split area: top section (metrics/widgets) | bottom section (processes full-width)
    let chunks = Layout::vertical([
        Constraint::Ratio(5, 9),  // Top (more space for widgets)
        Constraint::Ratio(4, 9),  // Bottom: processes
    ])
    .spacing(1)
    .split(area);

    // ── Top section: 3-column grid ──
    let cols = Layout::horizontal([
        Constraint::Ratio(35, 100),  // Left: CPU, Memory, Network
        Constraint::Ratio(35, 100),  // Center: Disk, GPU, System, Viz
        Constraint::Ratio(30, 100),  // Right: Clock, Media, Calendar
    ])
    .spacing(1)
    .split(chunks[0]);

    // ── Column 1: Hardware metrics ──
    let col1 = Layout::vertical([
        Constraint::Min(8),       // CPU
        Constraint::Length(5),    // Memory
        Constraint::Length(5),    // Network
    ])
    .spacing(0)
    .split(cols[0]);

    // ── Column 2: System / GPU / Disk / Viz ──
    let mut c2_constraints = vec![];
    if config.widgets.disk { c2_constraints.push(Constraint::Length(4)); }
    if config.widgets.gpu { c2_constraints.push(Constraint::Length(4)); }
    c2_constraints.push(Constraint::Length(9)); // System (always on)
    if config.widgets.music_viz { c2_constraints.push(Constraint::Min(0)); }

    let col2 = Layout::vertical(c2_constraints)
        .spacing(0)
        .split(cols[1]);

    // ── Column 3: Secondary widgets ──
    let mut c3_constraints = vec![];
    if config.widgets.clock { c3_constraints.push(Constraint::Length(7)); }
    if config.widgets.media { c3_constraints.push(Constraint::Length(3)); }
    if config.widgets.calendar { c3_constraints.push(Constraint::Length(10)); }
    if config.widgets.cmatrix { c3_constraints.push(Constraint::Min(0)); }
    c3_constraints.push(Constraint::Min(0)); // Spacer

    let col3 = Layout::vertical(c3_constraints)
        .spacing(1)
        .split(cols[2]);

    // ── Column 1: Hardware ──
    if config.widgets.cpu {
        let inner = section_header(f, col1[0], " CPU", theme, focused == Some(PanelId::Cpu));
        cpu::render(f, inner, theme);
    }
    if config.widgets.memory {
        let inner = section_header(f, col1[1], "󰍛 Memory", theme, focused == Some(PanelId::Memory));
        memory::render(f, inner, theme);
    }
    if config.widgets.network {
        let inner = section_header(f, col1[2], "󰤨 Network", theme, focused == Some(PanelId::Network));
        network::render(f, inner, theme);
    }

    // ── Column 2: System / Disk / GPU / Viz ──
    let mut ci2 = 0_usize;
    if config.widgets.disk {
        let inner = section_header(f, col2[ci2], "󰋊 Disk", theme, focused == Some(PanelId::Disk));
        disk::render(f, inner, theme);
        ci2 += 1;
    }
    if config.widgets.gpu {
        let inner = section_header(f, col2[ci2], "GPU", theme, focused == Some(PanelId::Gpu));
        gpu::render(f, inner, theme);
        ci2 += 1;
    }
    {
        let inner = section_header(f, col2[ci2], "System", theme, focused == Some(PanelId::System));
        system_info::render(f, inner, theme);
        ci2 += 1;
    }
    if config.widgets.music_viz {
        let inner = section_header(f, col2[ci2], "Visualizer", theme, focused == Some(PanelId::Visualizer));
        music_viz::render(f, inner, theme, tick);
    }

    // ── Column 3: Secondary (Clock/Media/Calendar) ──

    let mut ci3 = 0_usize;

    if config.widgets.clock {
        let inner = section_header(f, col3[ci3], "Clock", theme, focused == Some(PanelId::Clock));
        clock::render(f, inner, theme);
        ci3 += 1;
    }

    if config.widgets.media {
        let inner = section_header(f, col3[ci3], "Media", theme, focused == Some(PanelId::Media));
        media::render(f, inner, theme);
        ci3 += 1;
    }

    if config.widgets.calendar {
        let inner = section_header(f, col3[ci3], "Calendar", theme, focused == Some(PanelId::Calendar));
        calendar::render(f, inner, theme, states.calendar_month_offset);
        ci3 += 1;
    }

    if config.widgets.cmatrix {
        let inner = section_header(f, col3[ci3], "Matrix", theme, false);
        cmatrix::render(f, inner, tick);
    }

    // ── Bottom section: Processes (full width) ──

    if config.widgets.processes {
        let inner = section_header(f, chunks[1], "Processes", theme, focused == Some(PanelId::Processes));
        processes::render(
            f,
            inner,
            theme,
            states.process_scroll_offset,
            states.process_sort_field,
            states.process_sort_asc,
            &states.process_search,
            states.process_tree_mode,
            &states.process_collapsed,
            states.process_selected_pid,
            states.process_compact_cmd,
            states.process_show_detail,
        );
    }
}
