use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::Span;
use ratatui::Frame;

use crate::app::{self, PanelId, PanelStates};
use crate::config::Config;
use crate::monitors::{cpu, disk, gpu, memory, network, processes, system_info};
use crate::widgets::{calendar, clock, media, music_viz};

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
        Constraint::Ratio(4, 10),  // Primary: CPU, Memory, Disk, Network
        Constraint::Ratio(3, 10),  // Secondary: Clock, Calendar, Media
        Constraint::Ratio(3, 10),  // Peripheral: GPU, Visualizer, System
    ])
    .spacing(1)
    .split(chunks[0]);

    // ── Column 1: Primary monitoring (Combined Analytics Box) ──
    // We will render the section header first, then split its inner area
    // This removes the individual borders and makes them look like one cohesive block!

    // ── Column 2: Secondary widgets ──
    let col2 = if config.widgets.media {
        Layout::vertical([
            Constraint::Length(7),    // Clock (Big)
            Constraint::Length(3),    // Media (compact)
            Constraint::Length(10),   // Calendar
            Constraint::Min(0),       // Spacer
        ])
        .spacing(1)
        .split(cols[1])
    } else {
        Layout::vertical([
            Constraint::Length(7),    // Clock (Big)
            Constraint::Length(10),   // Calendar
            Constraint::Min(0),       // Spacer
        ])
        .spacing(1)
        .split(cols[1])
    };

    // ── Column 3: Peripheral ──
    let col3 = Layout::vertical([
        Constraint::Length(4),       // GPU
        Constraint::Length(7),       // System
        Constraint::Min(0),          // Visualizer (expands)
    ])
    .spacing(1)
    .split(cols[2]);

    // ── Column 1: Primary Monitoring (Analytics) ──
    // Determine if any of the widgets in this group are focused
    let analytics_focused = matches!(
        focused,
        Some(PanelId::Cpu) | Some(PanelId::Memory) | Some(PanelId::Disk) | Some(PanelId::Network)
    );
    let analytics_inner = section_header(f, cols[0], "System Analytics", theme, analytics_focused);

    // Split the inner area into the 4 widgets, keeping 1 line spacing between them
    let col1 = Layout::vertical([
        Constraint::Min(8),       // CPU
        Constraint::Length(3),    // Memory
        Constraint::Length(3),    // Disk
        Constraint::Length(3),    // Network
    ])
    .spacing(1)
    .split(analytics_inner);

    if config.widgets.cpu {
        cpu::render(f, col1[0], theme);
    }
    if config.widgets.memory {
        memory::render(f, col1[1], theme);
    }
    if config.widgets.disk {
        disk::render(f, col1[2], theme);
    }
    if config.widgets.network {
        network::render(f, col1[3], theme);
    }

    // ── Column 2: Secondary ──

    let mut ci = 0_usize;

    // Clock
    if config.widgets.clock {
        let inner = section_header(f, col2[ci], "Clock", theme, focused == Some(PanelId::Clock));
        clock::render(f, inner, theme);
        ci += 1;
    }

    // Media
    if config.widgets.media {
        let inner = section_header(f, col2[ci], "Media", theme, focused == Some(PanelId::Media));
        media::render(f, inner, theme);
        ci += 1;
    }

    // Calendar
    if config.widgets.calendar {
        let inner = section_header(f, col2[ci], "Calendar", theme, focused == Some(PanelId::Calendar));
        calendar::render(f, inner, theme, states.calendar_month_offset);
    }

    // ── Column 3: Peripheral ──

    // GPU
    if config.widgets.gpu {
        let inner = section_header(f, col3[0], "GPU", theme, focused == Some(PanelId::Gpu));
        gpu::render(f, inner, theme);
    }

    // System Info
    {
        let inner = section_header(f, col3[1], "System", theme, focused == Some(PanelId::System));
        system_info::render(f, inner, theme);
    }

    // Visualizer
    if config.widgets.music_viz {
        let inner = section_header(f, col3[2], "Visualizer", theme, focused == Some(PanelId::Visualizer));
        music_viz::render(f, inner, theme, tick);
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
