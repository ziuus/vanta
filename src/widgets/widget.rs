use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::Span;
use ratatui::widgets::{Block, BorderType, Borders};
use ratatui::Frame;

use crate::app::{PanelStates, Theme};
use crate::config::Config;
use crate::mode::DashboardMode;
use crate::monitors::{cpu, disk, gpu, memory, network, processes, system_info};
use crate::screens::settings;
use crate::widgets::{calendar, clock, cmatrix, media, music_viz};

/// A single dashboard widget that can render itself to a given area.
///
/// Each implementation wraps an existing module's render function.
/// No module-level changes required — just adapter structs.
pub trait DashboardWidget {
    /// Unique panel identifier matching PanelId naming.
    fn id(&self) -> &'static str;
    /// Human-readable label for the panel title.
    #[allow(dead_code)]
    fn label(&self) -> &'static str;
    /// Emoji / icon prefix for the panel title.
    #[allow(dead_code)]
    fn icon(&self) -> &'static str;
    /// Render the widget into the given area.
    fn render(
        &self,
        f: &mut Frame,
        area: Rect,
        theme: &Theme,
        config: &Config,
        states: &PanelStates,
        tick: u64,
    );
}

// ── Widget adapter structs ──

pub struct CpuWidget;
pub struct MemoryWidget;
pub struct DiskWidget;
pub struct NetworkWidget;
pub struct GpuWidget;
pub struct SystemWidget;
pub struct ClockWidget;
pub struct CalendarWidget;
pub struct MediaWidget;
pub struct MusicVizWidget;
pub struct ProcessesWidget;
pub struct CmatrixWidget;
pub struct SettingsWidget;

// ── Trait implementations ──

fn draw_border(f: &mut Frame, area: Rect, title: &str, theme: &Theme) -> Rect {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.dim))
        .title(Span::styled(
            format!(" {} ", title),
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        ));
    let inner = block.inner(area);
    f.render_widget(block, area);
    inner
}

impl DashboardWidget for CpuWidget {
    fn id(&self) -> &'static str {
        "cpu"
    }
    fn label(&self) -> &'static str {
        "CPU"
    }
    fn icon(&self) -> &'static str {
        "▂▁▃ "
    }
    fn render(
        &self,
        f: &mut Frame,
        area: Rect,
        theme: &Theme,
        _config: &Config,
        _states: &PanelStates,
        _tick: u64,
    ) {
        let inner = draw_border(f, area, self.label(), theme);
        cpu::render(f, inner, theme);
    }
}

impl DashboardWidget for MemoryWidget {
    fn id(&self) -> &'static str {
        "memory"
    }
    fn label(&self) -> &'static str {
        "Memory"
    }
    fn icon(&self) -> &'static str {
        "🧠 "
    }
    fn render(
        &self,
        f: &mut Frame,
        area: Rect,
        theme: &Theme,
        _config: &Config,
        _states: &PanelStates,
        _tick: u64,
    ) {
        let inner = draw_border(f, area, self.label(), theme);
        memory::render(f, inner, theme);
    }
}

impl DashboardWidget for DiskWidget {
    fn id(&self) -> &'static str {
        "disk"
    }
    fn label(&self) -> &'static str {
        "Disk"
    }
    fn icon(&self) -> &'static str {
        "💾 "
    }
    fn render(
        &self,
        f: &mut Frame,
        area: Rect,
        theme: &Theme,
        _config: &Config,
        _states: &PanelStates,
        _tick: u64,
    ) {
        let inner = draw_border(f, area, self.label(), theme);
        disk::render(f, inner, theme);
    }
}

impl DashboardWidget for NetworkWidget {
    fn id(&self) -> &'static str {
        "network"
    }
    fn label(&self) -> &'static str {
        "Network"
    }
    fn icon(&self) -> &'static str {
        "🌐 "
    }
    fn render(
        &self,
        f: &mut Frame,
        area: Rect,
        theme: &Theme,
        _config: &Config,
        _states: &PanelStates,
        _tick: u64,
    ) {
        let inner = draw_border(f, area, self.label(), theme);
        network::render(f, inner, theme);
    }
}

impl DashboardWidget for GpuWidget {
    fn id(&self) -> &'static str {
        "gpu"
    }
    fn label(&self) -> &'static str {
        "GPU"
    }
    fn icon(&self) -> &'static str {
        "🎮 "
    }
    fn render(
        &self,
        f: &mut Frame,
        area: Rect,
        theme: &Theme,
        _config: &Config,
        _states: &PanelStates,
        _tick: u64,
    ) {
        let inner = draw_border(f, area, self.label(), theme);
        gpu::render(f, inner, theme);
    }
}

impl DashboardWidget for SystemWidget {
    fn id(&self) -> &'static str {
        "system"
    }
    fn label(&self) -> &'static str {
        "System"
    }
    fn icon(&self) -> &'static str {
        "🖥 "
    }
    fn render(
        &self,
        f: &mut Frame,
        area: Rect,
        theme: &Theme,
        _config: &Config,
        _states: &PanelStates,
        _tick: u64,
    ) {
        let inner = draw_border(f, area, self.label(), theme);
        system_info::render(f, inner, theme);
    }
}

impl DashboardWidget for ClockWidget {
    fn id(&self) -> &'static str {
        "clock"
    }
    fn label(&self) -> &'static str {
        "Clock"
    }
    fn icon(&self) -> &'static str {
        "⏱ "
    }
    fn render(
        &self,
        f: &mut Frame,
        area: Rect,
        theme: &Theme,
        _config: &Config,
        _states: &PanelStates,
        _tick: u64,
    ) {
        let inner = draw_border(f, area, self.label(), theme);
        clock::render(f, inner, theme);
    }
}

impl DashboardWidget for CalendarWidget {
    fn id(&self) -> &'static str {
        "calendar"
    }
    fn label(&self) -> &'static str {
        "Calendar"
    }
    fn icon(&self) -> &'static str {
        "📅 "
    }
    fn render(
        &self,
        f: &mut Frame,
        area: Rect,
        theme: &Theme,
        _config: &Config,
        states: &PanelStates,
        _tick: u64,
    ) {
        let inner = draw_border(f, area, self.label(), theme);
        calendar::render(f, inner, theme, states.calendar_month_offset);
    }
}

impl DashboardWidget for MediaWidget {
    fn id(&self) -> &'static str {
        "media"
    }
    fn label(&self) -> &'static str {
        "Media"
    }
    fn icon(&self) -> &'static str {
        "🎵 "
    }
    fn render(
        &self,
        f: &mut Frame,
        area: Rect,
        theme: &Theme,
        _config: &Config,
        _states: &PanelStates,
        _tick: u64,
    ) {
        let inner = draw_border(f, area, self.label(), theme);
        media::render(f, inner, theme);
    }
}

impl DashboardWidget for MusicVizWidget {
    fn id(&self) -> &'static str {
        "music_viz"
    }
    fn label(&self) -> &'static str {
        "Visualizer"
    }
    fn icon(&self) -> &'static str {
        "🎵 "
    }
    fn render(
        &self,
        f: &mut Frame,
        area: Rect,
        theme: &Theme,
        _config: &Config,
        _states: &PanelStates,
        tick: u64,
    ) {
        let inner = draw_border(f, area, self.label(), theme);
        music_viz::render(f, inner, theme, tick);
    }
}

impl DashboardWidget for ProcessesWidget {
    fn id(&self) -> &'static str {
        "processes"
    }
    fn label(&self) -> &'static str {
        "Processes"
    }
    fn icon(&self) -> &'static str {
        "⚙ "
    }
    fn render(
        &self,
        f: &mut Frame,
        area: Rect,
        theme: &Theme,
        _config: &Config,
        states: &PanelStates,
        _tick: u64,
    ) {
        let inner = draw_border(f, area, self.label(), theme);
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

impl DashboardWidget for CmatrixWidget {
    fn id(&self) -> &'static str {
        "cmatrix"
    }
    fn label(&self) -> &'static str {
        "Cmatrix"
    }
    fn icon(&self) -> &'static str {
        "〰 "
    }
    fn render(
        &self,
        f: &mut Frame,
        area: Rect,
        _theme: &Theme,
        _config: &Config,
        _states: &PanelStates,
        tick: u64,
    ) {
        let inner = draw_border(f, area, self.label(), _theme);
        cmatrix::render(f, inner, tick, _theme);
    }
}

impl DashboardWidget for SettingsWidget {
    fn id(&self) -> &'static str {
        "settings"
    }
    fn label(&self) -> &'static str {
        "Settings"
    }
    fn icon(&self) -> &'static str {
        "⚙ "
    }
    fn render(
        &self,
        f: &mut Frame,
        area: Rect,
        theme: &Theme,
        config: &Config,
        _states: &PanelStates,
        _tick: u64,
    ) {
        let inner = draw_border(f, area, self.label(), theme);
        settings::render(f, inner, theme, config);
    }
}

// ── Registry ──

/// Returns the widget list active in each dashboard mode.
///
/// Phase 2: trait + adapter definitions only.
/// Phase 4 wires this into the render dispatch and applies config toggles.
pub fn widgets_for_mode(mode: DashboardMode) -> Vec<Box<dyn DashboardWidget>> {
    match mode {
        DashboardMode::Overview => vec![
            Box::new(CpuWidget),
            Box::new(MemoryWidget),
            Box::new(DiskWidget),
            Box::new(NetworkWidget),
            Box::new(GpuWidget),
            Box::new(SystemWidget),
            Box::new(ClockWidget),
            Box::new(CalendarWidget),
            Box::new(MediaWidget),
            Box::new(MusicVizWidget),
            Box::new(ProcessesWidget),
        ],
        DashboardMode::Monitor => vec![
            Box::new(CpuWidget),
            Box::new(MemoryWidget),
            Box::new(DiskWidget),
            Box::new(NetworkWidget),
            Box::new(GpuWidget),
            Box::new(SystemWidget),
            Box::new(ProcessesWidget),
        ],
        DashboardMode::Media => vec![
            Box::new(ClockWidget),
            Box::new(MusicVizWidget),
            Box::new(MediaWidget),
        ],
        DashboardMode::Aesthetic => vec![
            Box::new(ClockWidget),
            Box::new(CalendarWidget),
            Box::new(MusicVizWidget),
            Box::new(CmatrixWidget),
        ],
        DashboardMode::Processes | DashboardMode::Settings => Vec::new(),
    }
}
