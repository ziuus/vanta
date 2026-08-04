use ratatui::layout::{Constraint, Layout, Rect};

use crate::config::Config;
use crate::mode::DashboardMode;

/// A single widget placement: which widget renders in which screen area.
pub struct WidgetPlacement {
    pub id: &'static str,
    pub area: Rect,
}

/// Returns the widget placements for the given mode and available screen area.
///
/// Disabled widgets (per `config.widgets.*`) are omitted from the returned list.
/// Phase 4 uses this + `widgets_for_mode` to drive `app.render()` dispatch.
pub fn layout_for_mode(mode: DashboardMode, area: Rect, config: &Config) -> Vec<WidgetPlacement> {
    match mode {
        DashboardMode::Overview => overview_layout(area, config),
        DashboardMode::Monitor => monitor_layout(area),
        DashboardMode::Media => media_layout(area),
        DashboardMode::Aesthetic => aesthetic_layout(area),
        DashboardMode::Processes | DashboardMode::Settings => Vec::new(),
    }
}

// ── Monitor: Neohtop-style layout ──

fn monitor_layout(area: Rect) -> Vec<WidgetPlacement> {
    // Split into top half (metrics) and bottom half (processes)
    let chunks = Layout::vertical([
        Constraint::Ratio(2, 5), // Top half
        Constraint::Ratio(3, 5), // Bottom half
    ])
    .split(area);

    // Split top half into 3 columns
    let cols = Layout::horizontal([
        Constraint::Ratio(1, 3), // CPU
        Constraint::Ratio(1, 3), // Mem / Disk
        Constraint::Ratio(1, 3), // Net / Sys / GPU
    ])
    .split(chunks[0]);

    // CPU is full height in col 0
    let cpu_area = cols[0];

    // Col 1: Mem, Disk
    let col1 = Layout::vertical([
        Constraint::Length(4), // Memory
        Constraint::Length(4), // Disk
        Constraint::Min(0),
    ])
    .split(cols[1]);

    // Col 2: Net, GPU, System
    let col2 = Layout::vertical([
        Constraint::Length(4), // Net
        Constraint::Length(4), // GPU
        Constraint::Length(7), // System
        Constraint::Min(0),
    ])
    .split(cols[2]);

    vec![
        WidgetPlacement {
            id: "cpu",
            area: cpu_area,
        },
        WidgetPlacement {
            id: "memory",
            area: col1[0],
        },
        WidgetPlacement {
            id: "disk",
            area: col1[1],
        },
        WidgetPlacement {
            id: "network",
            area: col2[0],
        },
        WidgetPlacement {
            id: "gpu",
            area: col2[1],
        },
        WidgetPlacement {
            id: "system",
            area: col2[2],
        },
        WidgetPlacement {
            id: "processes",
            area: chunks[1],
        },
    ]
}

// ── Overview: exact replica of current overview.rs layout ──

fn overview_layout(area: Rect, config: &Config) -> Vec<WidgetPlacement> {
    let mut placements = Vec::with_capacity(11);

    let chunks = Layout::vertical([Constraint::Ratio(4, 9), Constraint::Ratio(5, 9)]).split(area);

    let cols = Layout::horizontal([
        Constraint::Ratio(4, 10),
        Constraint::Ratio(3, 10),
        Constraint::Ratio(3, 10),
    ])
    .split(chunks[0]);

    // Column 1: Primary monitoring
    let col1 = Layout::vertical([
        Constraint::Min(0),    // CPU expands to fill top space (good for multi-core)
        Constraint::Length(4), // Memory
        Constraint::Length(4), // Disk
        Constraint::Length(4), // Network
    ])
    .split(cols[0]);

    push_if(&mut placements, "cpu", col1[0], config.widgets.cpu);
    push_if(&mut placements, "memory", col1[1], config.widgets.memory);
    push_if(&mut placements, "disk", col1[2], config.widgets.disk);
    push_if(&mut placements, "network", col1[3], config.widgets.network);

    // Column 2: Secondary widgets
    let col2 = if config.widgets.media {
        Layout::vertical([
            Constraint::Length(5),  // Clock
            Constraint::Length(3),  // Media
            Constraint::Length(10), // Calendar
            Constraint::Min(0),
        ])
        .split(cols[1])
    } else {
        Layout::vertical([
            Constraint::Length(5),  // Clock
            Constraint::Length(10), // Calendar
            Constraint::Min(0),
        ])
        .split(cols[1])
    };

    let mut ci = 0_usize;
    if config.widgets.clock {
        placements.push(WidgetPlacement {
            id: "clock",
            area: col2[ci],
        });
        ci += 1;
    }
    if config.widgets.media {
        placements.push(WidgetPlacement {
            id: "media",
            area: col2[ci],
        });
        ci += 1;
    }
    if config.widgets.calendar {
        placements.push(WidgetPlacement {
            id: "calendar",
            area: col2[ci],
        });
    }

    // Column 3: Peripheral
    let col3 = Layout::vertical([
        Constraint::Length(4), // GPU
        Constraint::Length(7), // System
        Constraint::Min(0),    // Visualizer takes the rest
    ])
    .split(cols[2]);

    push_if(&mut placements, "gpu", col3[0], config.widgets.gpu);
    push_if(&mut placements, "system", col3[1], true);
    push_if(
        &mut placements,
        "music_viz",
        col3[2],
        config.widgets.music_viz,
    );

    // Bottom: processes
    if config.widgets.processes {
        placements.push(WidgetPlacement {
            id: "processes",
            area: chunks[1],
        });
    }

    placements
}

// ── Media: visualizer + player + clock ──

fn media_layout(area: Rect) -> Vec<WidgetPlacement> {
    let rows = Layout::vertical([
        Constraint::Length(5),   // Clock
        Constraint::Ratio(2, 3), // Visualizer
        Constraint::Ratio(1, 3), // Media player
    ])
    .split(area);

    vec![
        WidgetPlacement {
            id: "clock",
            area: rows[0],
        },
        WidgetPlacement {
            id: "music_viz",
            area: rows[1],
        },
        WidgetPlacement {
            id: "media",
            area: rows[2],
        },
    ]
}

// ── Aesthetic: clock + calendar + visualizer + cmatrix ──

fn aesthetic_layout(area: Rect) -> Vec<WidgetPlacement> {
    let rows = Layout::vertical([
        Constraint::Length(10), // Clock + Calendar height
        Constraint::Min(0),     // Visualizer + Cmatrix get the rest
    ])
    .split(area);

    let top = Layout::horizontal([Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)]).split(rows[0]);

    let bottom = Layout::horizontal([
        Constraint::Ratio(1, 3),
        Constraint::Ratio(1, 3),
        Constraint::Ratio(1, 3),
    ])
    .split(rows[1]);

    vec![
        WidgetPlacement {
            id: "clock",
            area: top[0],
        },
        WidgetPlacement {
            id: "calendar",
            area: top[1],
        },
        WidgetPlacement {
            id: "music_viz",
            area: bottom[0],
        },
        WidgetPlacement {
            id: "cmatrix",
            area: bottom[1],
        },
        WidgetPlacement {
            id: "video",
            area: bottom[2],
        },
    ]
}

// ── Helpers ──

fn push_if(placements: &mut Vec<WidgetPlacement>, id: &'static str, area: Rect, enabled: bool) {
    if enabled {
        placements.push(WidgetPlacement { id, area });
    }
}
