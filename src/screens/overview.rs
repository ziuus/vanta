use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::Span;
use ratatui::Frame;

use crate::app::{self, PanelId, PanelStates};
use crate::config::Config;
use crate::monitors::{cpu, disk, gpu, memory, network, processes, system_info};
use crate::widgets::{calendar, clock, cmatrix, media, music_viz};

use ratatui::widgets::{Block, Borders, BorderType};

/// Render a btop-style colored section label with a full border.
/// Returns the remaining area below the label for the widget content.
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

/// Helper to merge borders horizontally by extending width of all but the last chunk.
fn overlap_horizontal(chunks: &mut [Rect]) {
    for i in 0..chunks.len().saturating_sub(1) {
        chunks[i].width += 1;
    }
}

/// Helper to merge borders vertically by extending height of all but the last chunk.
fn overlap_vertical(chunks: &mut [Rect]) {
    for i in 0..chunks.len().saturating_sub(1) {
        chunks[i].height += 1;
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
    // ── Main Layout Split ──
    let mut chunks = Layout::vertical([
        Constraint::Ratio(5, 9),  // Top (more space for widgets)
        Constraint::Ratio(4, 9),  // Bottom: processes
    ])
    .spacing(0)
    .split(area)
    .to_vec();
    overlap_vertical(&mut chunks); // Merge top and bottom sections

    // ── Top section: 3-column grid ──
    let mut cols = Layout::horizontal([
        Constraint::Ratio(35, 100),  // Left: Hardware
        Constraint::Ratio(35, 100),  // Center: System
        Constraint::Ratio(30, 100),  // Right: Widgets
    ])
    .spacing(0)
    .split(chunks[0])
    .to_vec();
    overlap_horizontal(&mut cols); // Merge columns horizontally

    // ── Column 1: Hardware metrics ──
    let mut c1_constraints = vec![];
    if config.widgets.cpu { c1_constraints.push(Constraint::Min(8)); } // Expands
    if config.widgets.memory { c1_constraints.push(Constraint::Length(4)); } // h=4 -> expanded to 5 -> inner 3
    if config.widgets.disk { c1_constraints.push(Constraint::Length(4)); }   // h=4 -> inner 2 (since it's last, it doesn't expand, wait, if disk is last, we want inner 2 so length 4 is correct!)

    let mut col1 = Layout::vertical(c1_constraints).spacing(0).split(cols[0]).to_vec();
    overlap_vertical(&mut col1);

    // ── Column 2: System / Network / GPU / Matrix ──
    let mut c2_constraints = vec![];
    c2_constraints.push(Constraint::Length(8)); // System (h=8 -> expanded to 9 -> inner 7)
    if config.widgets.network { c2_constraints.push(Constraint::Length(4)); } // h=4 -> expanded to 5 -> inner 3
    if config.widgets.gpu { c2_constraints.push(Constraint::Length(4)); } // h=4 -> expanded to 5 -> inner 3
    if config.widgets.cmatrix { c2_constraints.push(Constraint::Min(0)); } 

    let mut col2 = Layout::vertical(c2_constraints).spacing(0).split(cols[1]).to_vec();
    overlap_vertical(&mut col2);

    // ── Column 3: Time / Media / Visualizer ──
    let mut c3_constraints = vec![];
    if config.widgets.clock { c3_constraints.push(Constraint::Length(6)); } // expanded to 7 -> inner 5
    if config.widgets.calendar { c3_constraints.push(Constraint::Length(9)); } // expanded to 10 -> inner 8
    if config.widgets.media { c3_constraints.push(Constraint::Length(3)); } // expanded to 4 -> inner 2
    if config.widgets.music_viz { c3_constraints.push(Constraint::Min(0)); }

    let mut col3 = Layout::vertical(c3_constraints).spacing(0).split(cols[2]).to_vec();
    overlap_vertical(&mut col3);

    // ── Column 1: Hardware ──
    let mut ci1 = 0_usize;
    if config.widgets.cpu {
        let inner = section_header(f, col1[ci1], " CPU", theme, focused == Some(PanelId::Cpu));
        cpu::render(f, inner, theme);
        ci1 += 1;
    }
    if config.widgets.memory {
        let inner = section_header(f, col1[ci1], "󰍛 Memory", theme, focused == Some(PanelId::Memory));
        memory::render(f, inner, theme);
        ci1 += 1;
    }
    if config.widgets.disk {
        let inner = section_header(f, col1[ci1], "󰋊 Disk", theme, focused == Some(PanelId::Disk));
        disk::render(f, inner, theme);
    }

    // ── Column 2: System / Network / GPU / Matrix ──
    let mut ci2 = 0_usize;
    {
        let inner = section_header(f, col2[ci2], "System", theme, focused == Some(PanelId::System));
        system_info::render(f, inner, theme);
        ci2 += 1;
    }
    if config.widgets.network {
        let inner = section_header(f, col2[ci2], "󰤨 Network", theme, focused == Some(PanelId::Network));
        network::render(f, inner, theme);
        ci2 += 1;
    }
    if config.widgets.gpu {
        let inner = section_header(f, col2[ci2], "GPU", theme, focused == Some(PanelId::Gpu));
        gpu::render(f, inner, theme);
        ci2 += 1;
    }
    if config.widgets.cmatrix {
        let inner = section_header(f, col2[ci2], "Matrix", theme, false);
        cmatrix::render(f, inner, tick);
    }

    // ── Column 3: Secondary (Clock/Calendar/Media/Visualizer) ──
    let mut ci3 = 0_usize;
    if config.widgets.clock {
        let inner = section_header(f, col3[ci3], "Clock", theme, focused == Some(PanelId::Clock));
        clock::render(f, inner, theme);
        ci3 += 1;
    }
    if config.widgets.calendar {
        let inner = section_header(f, col3[ci3], "Calendar", theme, focused == Some(PanelId::Calendar));
        calendar::render(f, inner, theme, states.calendar_month_offset);
        ci3 += 1;
    }
    if config.widgets.media {
        let inner = section_header(f, col3[ci3], "Media", theme, focused == Some(PanelId::Media));
        media::render(f, inner, theme);
        ci3 += 1;
    }
    if config.widgets.music_viz {
        let inner = section_header(f, col3[ci3], "Visualizer", theme, focused == Some(PanelId::Visualizer));
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
