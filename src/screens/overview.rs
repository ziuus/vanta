use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::Style;
use ratatui::widgets::{Block, BorderType, Borders};
use ratatui::Frame;

use crate::app::{self, PanelId, PanelStates};
use crate::config::Config;
use crate::monitors::{cpu, disk, gpu, memory, network, processes, system_info};
use crate::widgets::{calendar, clock, media, music_viz};

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
        Constraint::Ratio(4, 9),  // Top: metrics + widgets (slightly bigger for primary)
        Constraint::Ratio(5, 9),  // Bottom: processes
    ])
    .split(area);

    // ── Top section: 3-column grid, primary gets wider ──
    let cols = Layout::horizontal([
        Constraint::Ratio(4, 10),  // Primary: CPU, Memory, Disk, Network
        Constraint::Ratio(3, 10),  // Secondary: Clock, Calendar, Media
        Constraint::Ratio(3, 10),  // Peripheral: GPU, Visualizer, System
    ])
    .split(chunks[0]);

    // Column 1: Primary monitoring (4 stacked panels)
    let col1 = Layout::vertical([
        Constraint::Ratio(1, 4),  // CPU
        Constraint::Ratio(1, 4),  // Memory
        Constraint::Ratio(1, 4),  // Disk
        Constraint::Ratio(1, 4),  // Network
    ])
    .split(cols[0]);

    // Column 2: Secondary widgets
    let col2 = if config.widgets.media {
        Layout::vertical([
            Constraint::Ratio(2, 5),  // Clock (bigger)
            Constraint::Length(3),     // Media (compact)
            Constraint::Ratio(2, 5),  // Calendar (bigger)
        ])
        .split(cols[1])
    } else {
        Layout::vertical([
            Constraint::Ratio(1, 2),  // Clock
            Constraint::Ratio(1, 2),  // Calendar
        ])
        .split(cols[1])
    };

    // Column 3: GPU + Visualizer + System Info
    let col3 = Layout::vertical([
        Constraint::Ratio(1, 3),
        Constraint::Ratio(1, 3),
        Constraint::Ratio(1, 3),
    ])
    .split(cols[2]);

    // ── Column 1: Primary Monitoring ──

    // CPU
    if config.widgets.cpu {
        let is_focused = focused == Some(PanelId::Cpu);
        let block = panel(" CPU ", "▂▁▃ ", theme, is_focused);
        let inner = block.inner(col1[0]);
        f.render_widget(block, col1[0]);
        cpu::render(f, inner, theme, config.demo);
    }

    // Memory
    if config.widgets.memory {
        let is_focused = focused == Some(PanelId::Memory);
        let block = panel(" Memory ", "🧠 ", theme, is_focused);
        let inner = block.inner(col1[1]);
        f.render_widget(block, col1[1]);
        memory::render(f, inner, theme, config.demo);
    }

    // Disk
    if config.widgets.disk {
        let is_focused = focused == Some(PanelId::Disk);
        let block = panel(" Disk ", "💾 ", theme, is_focused);
        let inner = block.inner(col1[2]);
        f.render_widget(block, col1[2]);
        disk::render(f, inner, theme, config.demo);
    }

    // Network
    if config.widgets.network {
        let is_focused = focused == Some(PanelId::Network);
        let block = panel(" Network ", "🌐 ", theme, is_focused);
        let inner = block.inner(col1[3]);
        f.render_widget(block, col1[3]);
        network::render(f, inner, theme, config.demo);
    }

    // ── Column 2: Secondary ──

    let mut ci = 0_usize;

    // Clock
    if config.widgets.clock {
        let is_focused = focused == Some(PanelId::Clock);
        let block = panel(" Clock ", "⏱ ", theme, is_focused);
        let inner = block.inner(col2[ci]);
        f.render_widget(block, col2[ci]);
        clock::render(f, inner, theme, config.demo);
        ci += 1;
    }

    // Media
    if config.widgets.media {
        let is_focused = focused == Some(PanelId::Media);
        let block = panel(" Media ", "🎵 ", theme, is_focused);
        let inner = block.inner(col2[ci]);
        f.render_widget(block, col2[ci]);
        media::render(f, inner, theme, config.demo);
        ci += 1;
    }

    // Calendar
    if config.widgets.calendar {
        let is_focused = focused == Some(PanelId::Calendar);
        let block = panel(" Calendar ", "📅 ", theme, is_focused);
        let inner = block.inner(col2[ci]);
        f.render_widget(block, col2[ci]);
        calendar::render(f, inner, theme, states.calendar_month_offset);
    }

    // ── Column 3: Peripheral ──

    // GPU
    if config.widgets.gpu {
        let is_focused = focused == Some(PanelId::Gpu);
        let block = panel(" GPU ", "🎮 ", theme, is_focused);
        let inner = block.inner(col3[0]);
        f.render_widget(block, col3[0]);
        gpu::render(f, inner, theme, config.demo);
    }

    // Visualizer
    if config.widgets.music_viz {
        let is_focused = focused == Some(PanelId::Visualizer);
        let block = panel(" Visualizer ", "🎵 ", theme, is_focused);
        let inner = block.inner(col3[1]);
        f.render_widget(block, col3[1]);
        music_viz::render(f, inner, theme, tick);
    }

    // System Info
    {
        let is_focused = focused == Some(PanelId::System);
        let block = panel(" System ", "🖥 ", theme, is_focused);
        let inner = block.inner(col3[2]);
        f.render_widget(block, col3[2]);
        system_info::render(f, inner, theme, config.demo);
    }

    // ── Bottom section: Processes (full width) ──

    if config.widgets.processes {
        let is_focused = focused == Some(PanelId::Processes);
        let block = panel(" Processes ", "⚙ ", theme, is_focused);
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
            config.demo,
        );
    }
}

fn panel<'a>(title: &'a str, icon: &'a str, theme: &app::Theme, focused: bool) -> Block<'a> {
    let full_title = format!(" {}{}", icon, title);
    let border_color = if focused { theme.focus } else { theme.dim };
    let border_type = if focused {
        BorderType::Thick
    } else {
        BorderType::Plain
    };
    Block::default()
        .title(full_title)
        .title_style(Style::default().fg(theme.accent))
        .borders(Borders::ALL)
        .border_type(border_type)
        .border_style(Style::default().fg(border_color))
        .style(Style::default().bg(theme.surface))
}