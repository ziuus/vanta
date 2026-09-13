use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Paragraph};
use ratatui::Frame;

use crate::app::App;

fn centered(area: Rect, w: u16, h: u16) -> Rect {
    let w = w.min(area.width);
    let h = h.min(area.height);
    Rect::new(
        area.x + (area.width - w) / 2,
        area.y + (area.height - h) / 2,
        w,
        h,
    )
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SettingType {
    Theme,
    GaugeStyle,
    GraphStyle,
    Visualizer,
    RefreshRate,
    Fps,
    Clock24h,
    WidgetCpu,
    WidgetMemory,
    WidgetDisk,
    WidgetNetwork,
    WidgetGpu,
    WidgetClock,
    WidgetCalendar,
    WidgetMusicViz,
    WidgetProcesses,
    WidgetMedia,
    WidgetMatrix,
    WidgetVideo,
}

pub const SETTINGS_ITEMS: &[(SettingType, &str)] = &[
    (SettingType::Theme, "Theme"),
    (SettingType::GaugeStyle, "Gauge Style"),
    (SettingType::GraphStyle, "Graph Style"),
    (SettingType::Visualizer, "Visualizer"),
    (SettingType::RefreshRate, "Refresh Rate (s)"),
    (SettingType::Fps, "FPS"),
    (SettingType::Clock24h, "24h Clock"),
    (SettingType::WidgetCpu, "Show CPU"),
    (SettingType::WidgetMemory, "Show Memory"),
    (SettingType::WidgetDisk, "Show Disk"),
    (SettingType::WidgetNetwork, "Show Network"),
    (SettingType::WidgetGpu, "Show GPU"),
    (SettingType::WidgetClock, "Show Clock"),
    (SettingType::WidgetCalendar, "Show Calendar"),
    (SettingType::WidgetMusicViz, "Show Visualizer"),
    (SettingType::WidgetProcesses, "Show Processes"),
    (SettingType::WidgetMedia, "Show Media"),
    (SettingType::WidgetMatrix, "Show Matrix"),
    (SettingType::WidgetVideo, "Show Video"),
];

pub fn render(f: &mut Frame, area: Rect, app: &App) {
    let theme = &app.theme;
    let bg = theme.surface;
    let base = Style::default().bg(bg);
    let mut lines = vec![
        Line::from(vec![
            Span::styled("   ↑ ↓ ", base.fg(theme.dim)),
            Span::styled("navigate   ", base.fg(theme.text)),
            Span::styled("← → / enter ", base.fg(theme.dim)),
            Span::styled("change   ", base.fg(theme.text)),
            Span::styled("esc ", base.fg(theme.dim)),
            Span::styled("close", base.fg(theme.text)),
        ]),
        Line::from(Span::styled("", base)),
    ];

    let w = 54;
    
    // Add settings items
    let start_idx = app.settings_scroll;
    let max_visible = 18; // approx max rows to fit
    
    for (i, (stype, label)) in SETTINGS_ITEMS.iter().enumerate() {
        if i < start_idx || i >= start_idx + max_visible {
            continue;
        }
        let is_selected = i == app.settings_row;
        let line_style = if is_selected {
            base.fg(theme.bg).bg(theme.accent)
        } else {
            base
        };
        
        let val_str = match stype {
            SettingType::Theme => app.config.ui.theme.clone(),
            SettingType::GaugeStyle => app.config.ui.gauge_style.clone(),
            SettingType::GraphStyle => app.config.ui.graph_style.clone(),
            SettingType::Visualizer => app.config.ui.visualizer.clone(),
            SettingType::RefreshRate => format!("{:.1}", app.config.ui.refresh_rate),
            SettingType::Fps => app.config.ui.fps.to_string(),
            SettingType::Clock24h => if app.config.ui.clock_24h { "yes".to_string() } else { "no".to_string() },
            SettingType::WidgetCpu => if app.config.widgets.cpu { "on".to_string() } else { "off".to_string() },
            SettingType::WidgetMemory => if app.config.widgets.memory { "on".to_string() } else { "off".to_string() },
            SettingType::WidgetDisk => if app.config.widgets.disk { "on".to_string() } else { "off".to_string() },
            SettingType::WidgetNetwork => if app.config.widgets.network { "on".to_string() } else { "off".to_string() },
            SettingType::WidgetGpu => if app.config.widgets.gpu { "on".to_string() } else { "off".to_string() },
            SettingType::WidgetClock => if app.config.widgets.clock { "on".to_string() } else { "off".to_string() },
            SettingType::WidgetCalendar => if app.config.widgets.calendar { "on".to_string() } else { "off".to_string() },
            SettingType::WidgetMusicViz => if app.config.widgets.music_viz { "on".to_string() } else { "off".to_string() },
            SettingType::WidgetProcesses => if app.config.widgets.processes { "on".to_string() } else { "off".to_string() },
            SettingType::WidgetMedia => if app.config.widgets.media { "on".to_string() } else { "off".to_string() },
            SettingType::WidgetMatrix => if app.config.widgets.matrix { "on".to_string() } else { "off".to_string() },
            SettingType::WidgetVideo => if app.config.widgets.video { "on".to_string() } else { "off".to_string() },
        };
        
        let inner_w = (w - 2) as usize;
        let padding = inner_w.saturating_sub(label.len() + val_str.len() + 8);
        let pad_str = " ".repeat(padding);
        
        lines.push(Line::from(vec![
            Span::styled(if is_selected { " > " } else { "   " }, line_style.add_modifier(Modifier::BOLD)),
            Span::styled(format!("{} ", label), line_style),
            Span::styled(pad_str, line_style),
            Span::styled(format!(" {}   ", val_str), line_style.add_modifier(Modifier::BOLD)),
        ]));
    }

    let box_area = centered(area, w, lines.len() as u16 + 2);
    f.render_widget(Clear, box_area);
    f.render_widget(
        Paragraph::new(lines).style(base).block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(theme.accent))
                .title(Span::styled(
                    " settings ",
                    Style::default()
                        .fg(theme.accent)
                        .add_modifier(Modifier::BOLD),
                ))
                .style(base),
        ),
        box_area,
    );
}

pub fn handle_key(app: &mut App, key: crossterm::event::KeyCode) {
    use crossterm::event::KeyCode::*;
    match key {
        Up | Char('k') => {
            if app.settings_row > 0 {
                app.settings_row -= 1;
            } else {
                app.settings_row = SETTINGS_ITEMS.len() - 1;
            }
            if app.settings_row < app.settings_scroll {
                app.settings_scroll = app.settings_row;
            } else if app.settings_row >= app.settings_scroll + 18 {
                app.settings_scroll = app.settings_row.saturating_sub(17);
            }
        }
        Down | Char('j') => {
            if app.settings_row + 1 < SETTINGS_ITEMS.len() {
                app.settings_row += 1;
            } else {
                app.settings_row = 0;
            }
            if app.settings_row >= app.settings_scroll + 18 {
                app.settings_scroll = app.settings_row.saturating_sub(17);
            } else if app.settings_row < app.settings_scroll {
                app.settings_scroll = app.settings_row;
            }
        }
        Left | Char('h') => change_setting(app, false),
        Right | Char('l') | Enter => change_setting(app, true),
        Esc | Char('q') | Char('S') => app.show_settings = false,
        _ => {}
    }
}

fn change_setting(app: &mut App, forward: bool) {
    let (stype, _) = SETTINGS_ITEMS[app.settings_row];
    match stype {
        SettingType::Theme => {
            app.cycle_theme();
            // cycle_theme already saves and toasts, but we want modal to stay open 
            // Theme cycling goes forward only via cycle_theme? 
            // Wait, we can implement backwards by iterating themes, but cycle_theme is sufficient
        }
        SettingType::GaugeStyle => {
            crate::widgets::gauge::cycle_style();
            app.config.ui.gauge_style = crate::widgets::gauge::style_name().to_string();
        }
        SettingType::GraphStyle => {
            crate::widgets::block_graph::cycle_style();
            app.config.ui.graph_style = crate::widgets::block_graph::style_name().to_string();
        }
        SettingType::Visualizer => {
            crate::widgets::music_viz::cycle_style();
            app.config.ui.visualizer = crate::widgets::music_viz::style_name().to_string();
        }
        SettingType::RefreshRate => {
            app.adjust_refresh(forward);
        }
        SettingType::Fps => {
            let cur = app.config.ui.fps;
            let next = if forward { cur + 5 } else { cur.saturating_sub(5) };
            app.config.ui.fps = next.clamp(5, 120);
        }
        SettingType::Clock24h => app.config.ui.clock_24h = !app.config.ui.clock_24h,
        SettingType::WidgetCpu => app.config.widgets.cpu = !app.config.widgets.cpu,
        SettingType::WidgetMemory => app.config.widgets.memory = !app.config.widgets.memory,
        SettingType::WidgetDisk => app.config.widgets.disk = !app.config.widgets.disk,
        SettingType::WidgetNetwork => app.config.widgets.network = !app.config.widgets.network,
        SettingType::WidgetGpu => app.config.widgets.gpu = !app.config.widgets.gpu,
        SettingType::WidgetClock => app.config.widgets.clock = !app.config.widgets.clock,
        SettingType::WidgetCalendar => app.config.widgets.calendar = !app.config.widgets.calendar,
        SettingType::WidgetMusicViz => app.config.widgets.music_viz = !app.config.widgets.music_viz,
        SettingType::WidgetProcesses => app.config.widgets.processes = !app.config.widgets.processes,
        SettingType::WidgetMedia => app.config.widgets.media = !app.config.widgets.media,
        SettingType::WidgetMatrix => app.config.widgets.matrix = !app.config.widgets.matrix,
        SettingType::WidgetVideo => app.config.widgets.video = !app.config.widgets.video,
    }
    app.config.save();
}
