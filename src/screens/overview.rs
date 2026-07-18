use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::Span;
use ratatui::Frame;

use crate::app::{self, PanelId, PanelStates};
use crate::config::Config;
use crate::monitors::{cpu, disk, gpu, memory, network, processes, system_info};
use crate::widgets::{calendar, clock, cmatrix, media, music_viz};

use ratatui::widgets::{Block, Borders, BorderType};

/// Render a master container block with a border around the entire column.
fn container_block(f: &mut Frame, area: Rect, title: &str, theme: &app::Theme, focused: bool) -> Rect {
    let title_color = if focused { theme.focus } else { theme.accent };
    let border_color = if focused { theme.focus } else { theme.dim };
    
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color))
        .title(Span::styled(
            format!(" {} ", title),
            Style::default().fg(title_color).add_modifier(Modifier::BOLD),
        ));
        
    let inner = block.inner(area);
    f.render_widget(block, area);
    inner
}

/// Render an internal section header with NO vertical borders, just a title and an optional top line divider.
fn internal_header(f: &mut Frame, area: Rect, label: &str, theme: &app::Theme, focused: bool, is_first: bool) -> Rect {
    let title_color = if focused { theme.focus } else { theme.accent };
    
    // We can use a Block with Borders::TOP to act as a separator if it's not the first item.
    let block = if is_first {
        Block::default()
            .title(Span::styled(
                format!("{} ", label),
                Style::default().fg(title_color).add_modifier(Modifier::BOLD),
            ))
    } else {
        let border_color = theme.dim;
        Block::default()
            .borders(Borders::TOP)
            .border_type(BorderType::Plain)
            .border_style(Style::default().fg(border_color))
            .title(Span::styled(
                format!("{} ", label),
                Style::default().fg(title_color).add_modifier(Modifier::BOLD),
            ))
    };
        
    f.render_widget(block, area);
    
    // Manually calculate the inner area to guarantee we don't overlap the title.
    // The title always takes 1 line.
    Rect {
        x: area.x,
        y: area.y + 1,
        width: area.width,
        height: area.height.saturating_sub(1),
    }
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
    .spacing(0)
    .split(area);

    // ── Top section: 3-column grid ──
    let cols = Layout::horizontal([
        Constraint::Ratio(35, 100),  // Left: Hardware
        Constraint::Ratio(35, 100),  // Center: System
        Constraint::Ratio(30, 100),  // Right: Widgets
    ])
    .spacing(0) // No spacing between master columns, their borders will touch.
    .split(chunks[0]);

    // Check if any child in the column is focused
    let c1_focused = matches!(focused, Some(PanelId::Cpu | PanelId::Memory | PanelId::Disk));
    let c2_focused = matches!(focused, Some(PanelId::System | PanelId::Network | PanelId::Gpu));
    let c3_focused = matches!(focused, Some(PanelId::Clock | PanelId::Calendar | PanelId::Media | PanelId::Visualizer));

    let col1_area = container_block(f, cols[0], "Hardware", theme, c1_focused);
    let col2_area = container_block(f, cols[1], "System", theme, c2_focused);
    let col3_area = container_block(f, cols[2], "Dashboard", theme, c3_focused);

    // ── Column 1: Hardware metrics ──
    let mut c1_constraints = vec![];
    if config.widgets.cpu { c1_constraints.push(Constraint::Min(8)); } // CPU expands
    if config.widgets.memory { c1_constraints.push(Constraint::Length(5)); } // Mem takes 5 inner lines (includes 1 for header)
    if config.widgets.disk { c1_constraints.push(Constraint::Length(4)); }

    let col1 = Layout::vertical(c1_constraints).split(col1_area);

    // ── Column 2: System / Network / GPU / Matrix ──
    let mut c2_constraints = vec![];
    c2_constraints.push(Constraint::Length(9)); // System (always on)
    if config.widgets.network { c2_constraints.push(Constraint::Length(4)); }
    if config.widgets.gpu { c2_constraints.push(Constraint::Length(4)); }
    if config.widgets.cmatrix { c2_constraints.push(Constraint::Min(0)); } 

    let col2 = Layout::vertical(c2_constraints).split(col2_area);

    // ── Column 3: Time / Media / Visualizer ──
    let mut c3_constraints = vec![];
    if config.widgets.clock { c3_constraints.push(Constraint::Length(6)); }
    if config.widgets.calendar { c3_constraints.push(Constraint::Length(9)); }
    if config.widgets.media { c3_constraints.push(Constraint::Length(3)); }
    if config.widgets.music_viz { c3_constraints.push(Constraint::Min(0)); }

    let col3 = Layout::vertical(c3_constraints).split(col3_area);

    // ── Column 1: Hardware ──
    let mut ci1 = 0_usize;
    if config.widgets.cpu {
        let inner = internal_header(f, col1[ci1], " CPU", theme, focused == Some(PanelId::Cpu), ci1 == 0);
        cpu::render(f, inner, theme);
        ci1 += 1;
    }
    if config.widgets.memory {
        let inner = internal_header(f, col1[ci1], "󰍛 Memory", theme, focused == Some(PanelId::Memory), ci1 == 0);
        memory::render(f, inner, theme);
        ci1 += 1;
    }
    if config.widgets.disk {
        let inner = internal_header(f, col1[ci1], "󰋊 Disk", theme, focused == Some(PanelId::Disk), ci1 == 0);
        disk::render(f, inner, theme);
    }

    // ── Column 2: System / Network / GPU / Matrix ──
    let mut ci2 = 0_usize;
    {
        let inner = internal_header(f, col2[ci2], "System", theme, focused == Some(PanelId::System), ci2 == 0);
        system_info::render(f, inner, theme);
        ci2 += 1;
    }
    if config.widgets.network {
        let inner = internal_header(f, col2[ci2], "󰤨 Network", theme, focused == Some(PanelId::Network), ci2 == 0);
        network::render(f, inner, theme);
        ci2 += 1;
    }
    if config.widgets.gpu {
        let inner = internal_header(f, col2[ci2], "GPU", theme, focused == Some(PanelId::Gpu), ci2 == 0);
        gpu::render(f, inner, theme);
        ci2 += 1;
    }
    if config.widgets.cmatrix {
        let inner = internal_header(f, col2[ci2], "Matrix", theme, false, ci2 == 0);
        cmatrix::render(f, inner, tick);
    }

    // ── Column 3: Secondary (Clock/Calendar/Media/Visualizer) ──
    let mut ci3 = 0_usize;
    if config.widgets.clock {
        let inner = internal_header(f, col3[ci3], "Clock", theme, focused == Some(PanelId::Clock), ci3 == 0);
        clock::render(f, inner, theme);
        ci3 += 1;
    }
    if config.widgets.calendar {
        let inner = internal_header(f, col3[ci3], "Calendar", theme, focused == Some(PanelId::Calendar), ci3 == 0);
        calendar::render(f, inner, theme, states.calendar_month_offset);
        ci3 += 1;
    }
    if config.widgets.media {
        let inner = internal_header(f, col3[ci3], "Media", theme, focused == Some(PanelId::Media), ci3 == 0);
        media::render(f, inner, theme);
        ci3 += 1;
    }
    if config.widgets.music_viz {
        let inner = internal_header(f, col3[ci3], "Visualizer", theme, focused == Some(PanelId::Visualizer), ci3 == 0);
        music_viz::render(f, inner, theme, tick);
    }

    // ── Bottom section: Processes (full width) ──

    if config.widgets.processes {
        let p_focused = focused == Some(PanelId::Processes);
        let title_color = if p_focused { theme.focus } else { theme.accent };
        let border_color = if p_focused { theme.focus } else { theme.dim };
        
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(border_color))
            .title(Span::styled(
                " Processes ",
                Style::default().fg(title_color).add_modifier(Modifier::BOLD),
            ));
            
        let inner = block.inner(chunks[1]);
        f.render_widget(block, chunks[1]);
        
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
