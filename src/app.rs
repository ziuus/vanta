use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use std::collections::HashSet;
use std::fs;
use std::process;

use crate::config::Config;
use crate::monitors::processes;
use crate::screens::overview;

/// Summary metrics collected once per render for the top bar
struct Summary {
    cpu_pct: f32,
    mem_pct: f64,
    gpu_pct: u64,
    net_dl: String,
    net_ul: String,
    bat_pct: Option<u8>,
    uptime: String,
    os: String,
}

fn collect_summary() -> Summary {
    // CPU
    let mut system = sysinfo::System::new_all();
    system.refresh_cpu_all();
    let cpu_pct = system.global_cpu_usage();

    // Memory
    system.refresh_memory();
    let mem_pct = if system.total_memory() > 0 {
        (system.used_memory() as f64 / system.total_memory() as f64) * 100.0
    } else {
        0.0
    };

    // GPU via nvidia-smi
    let gpu_pct = std::process::Command::new("nvidia-smi")
        .args(["--query-gpu=utilization.gpu", "--format=csv,noheader,nounits"])
        .output()
        .ok()
        .and_then(|o| {
            let s = String::from_utf8_lossy(&o.stdout).trim().to_string();
            s.parse::<u64>().ok()
        })
        .unwrap_or(0);

    // Network (aggregate dl/ul from /sys/class/net)
    let (mut rx_total, mut tx_total) = (0u64, 0u64);
    if let Ok(dir) = fs::read_dir("/sys/class/net/") {
        for entry in dir.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name == "lo" { continue; }
            if let Ok(v) = fs::read_to_string(entry.path().join("statistics").join("rx_bytes")) {
                rx_total += v.trim().parse::<u64>().unwrap_or(0);
            }
            if let Ok(v) = fs::read_to_string(entry.path().join("statistics").join("tx_bytes")) {
                tx_total += v.trim().parse::<u64>().unwrap_or(0);
            }
        }
    }
    fn fmt_bytes(b: u64) -> String {
        if b > 1_000_000_000 {
            format!("{:.1}GB", b as f64 / 1_000_000_000.0)
        } else if b > 1_000_000 {
            format!("{:.1}MB", b as f64 / 1_000_000.0)
        } else if b > 1_000 {
            format!("{}KB", b / 1_000)
        } else {
            format!("{}B", b)
        }
    }

    // Battery
    let bat_pct = fs::read_to_string("/sys/class/power_supply/BAT0/capacity")
        .ok()
        .and_then(|s| s.trim().parse::<u8>().ok());

    // Uptime
    let uptime_secs = sysinfo::System::uptime();
    let h = uptime_secs / 3600;
    let m = (uptime_secs % 3600) / 60;
    let uptime_s = if h > 24 {
        format!("{}d{}h", h / 24, h % 24)
    } else {
        format!("{}h{}m", h, m)
    };

    // OS name
    let os = fs::read_to_string("/etc/os-release")
        .ok()
        .and_then(|content| {
            for line in content.lines() {
                if let Some(val) = line.strip_prefix("PRETTY_NAME=\"") {
                    return Some(val.trim_end_matches('"').to_string());
                }
            }
            None
        })
        .unwrap_or_else(|| {
            fs::read_to_string("/proc/sys/kernel/ostype")
                .ok()
                .map(|s| s.trim().to_string())
                .unwrap_or_else(|| "Linux".to_string())
        });

    Summary {
        cpu_pct,
        mem_pct,
        gpu_pct,
        net_dl: fmt_bytes(rx_total),
        net_ul: fmt_bytes(tx_total),
        bat_pct,
        uptime: uptime_s,
        os,
    }
}

/// Sort field for the processes panel
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SortField {
    Mem,
    Pid,
    Name,
    Cpu,
}

impl SortField {
    pub fn label(&self) -> &'static str {
        match self {
            SortField::Cpu => "CPU",
            SortField::Mem => "MEM",
            SortField::Pid => "PID",
            SortField::Name => "NAME",
        }
    }
    pub fn next(&self) -> Self {
        match self {
            SortField::Cpu => SortField::Mem,
            SortField::Mem => SortField::Pid,
            SortField::Pid => SortField::Name,
            SortField::Name => SortField::Cpu,
        }
    }
}

/// Panels in Tab-cycle order (row-major: left→right, top→bottom)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanelId {
    Cpu,
    Clock,
    Memory,
    Calendar,
    Visualizer,
    Network,
    Disk,
    Gpu,
    Processes,
    Media,
    System,
}

impl PanelId {
    /// Ordered list for Tab-cycle, excluding disabled widgets
    pub fn all(config: &Config) -> Vec<PanelId> {
        let mut v = Vec::with_capacity(10);
        if config.widgets.cpu { v.push(PanelId::Cpu); }
        if config.widgets.clock { v.push(PanelId::Clock); }
        if config.widgets.memory { v.push(PanelId::Memory); }
        if config.widgets.gpu { v.push(PanelId::Gpu); }
        if config.widgets.calendar { v.push(PanelId::Calendar); }
        if config.widgets.music_viz { v.push(PanelId::Visualizer); }
        if config.widgets.network { v.push(PanelId::Network); }
        if config.widgets.disk { v.push(PanelId::Disk); }
        if config.widgets.processes { v.push(PanelId::Processes); }
        if config.widgets.media { v.push(PanelId::Media); }
        v.push(PanelId::System);
        v
    }
}

/// Per-panel interactive state
#[derive(Debug, Clone)]
pub struct PanelStates {
    pub calendar_month_offset: i32,
    pub process_scroll_offset: usize,
    pub process_sort_field: SortField,
    pub process_sort_asc: bool,
    pub process_search: String,
    pub process_search_active: bool,
    pub process_tree_mode: bool,
    pub process_collapsed: HashSet<u32>,
}

impl PanelStates {
    fn new() -> Self {
        Self {
            calendar_month_offset: 0,
            process_scroll_offset: 0,
            process_sort_field: SortField::Mem,
            process_sort_asc: false,
            process_search: String::new(),
            process_search_active: false,
            process_tree_mode: false,
            process_collapsed: HashSet::new(),
        }
    }
}

#[derive(Clone)]
pub struct Theme {
    pub bg: Color,
    pub accent: Color,
    pub secondary: Color,
    pub surface: Color,
    pub text: Color,
    pub dim: Color,
    pub green: Color,
    pub yellow: Color,
    pub red: Color,
    /// Border colour when this panel has focus
    pub focus: Color,
}

impl Theme {
    pub fn dark() -> Self {
        Self {
            bg: Color::Rgb(10, 10, 15),
            accent: Color::Rgb(80, 200, 120),
            secondary: Color::Rgb(100, 120, 200),
            surface: Color::Rgb(20, 20, 30),
            text: Color::Rgb(220, 220, 230),
            dim: Color::Rgb(80, 80, 95),
            green: Color::Rgb(80, 200, 120),
            yellow: Color::Rgb(220, 200, 60),
            red: Color::Rgb(220, 80, 80),
            focus: Color::Rgb(120, 220, 160),
        }
    }

    pub fn light() -> Self {
        Self {
            bg: Color::Rgb(245, 245, 240),
            accent: Color::Rgb(40, 160, 80),
            secondary: Color::Rgb(60, 80, 180),
            surface: Color::Rgb(230, 230, 225),
            text: Color::Rgb(20, 20, 30),
            dim: Color::Rgb(140, 140, 145),
            green: Color::Rgb(40, 160, 80),
            yellow: Color::Rgb(180, 160, 20),
            red: Color::Rgb(200, 50, 50),
            focus: Color::Rgb(40, 180, 100),
        }
    }
}

pub struct App {
    pub running: bool,
    pub config: Config,
    pub theme: Theme,
    pub tick_count: u64,
    pub focused_panel: Option<PanelId>,
    pub panel_states: PanelStates,
}

impl App {
    pub fn new(config: Config) -> Self {
        Self {
            running: true,
            theme: Theme::dark(),
            config,
            tick_count: 0,
            focused_panel: None,
            panel_states: PanelStates::new(),
        }
    }

    pub fn toggle_theme(&mut self) {
        self.theme = if matches!(self.theme.bg, Color::Rgb(10, 10, 15)) {
            Theme::light()
        } else {
            Theme::dark()
        };
    }

    /// Cycle focus to the next/previous panel in Tab order
    pub fn cycle_focus(&mut self, forward: bool) {
        let panels = PanelId::all(&self.config);
        if panels.is_empty() {
            return;
        }
        let idx = self.focused_panel.and_then(|p| panels.iter().position(|&x| x == p));
        let next = match idx {
            Some(i) => {
                if forward {
                    (i + 1) % panels.len()
                } else {
                    (i + panels.len() - 1) % panels.len()
                }
            }
            None => 0,
        };
        self.focused_panel = Some(panels[next]);
    }

    /// Handle keys for the currently focused panel
    pub fn handle_panel_nav(&mut self, key: crossterm::event::KeyCode) {
        use crossterm::event::KeyCode;

        let Some(panel) = self.focused_panel else {
            return;
        };
        match panel {
            PanelId::Calendar => match key {
                KeyCode::Left => {
                    self.panel_states.calendar_month_offset -= 1;
                }
                KeyCode::Right => {
                    self.panel_states.calendar_month_offset += 1;
                }
                KeyCode::Home => {
                    self.panel_states.calendar_month_offset = 0;
                }
                _ => {}
            },
            PanelId::Processes => match key {
                // Tree mode toggle
                KeyCode::Char('t') | KeyCode::Char('T') => {
                    if !self.panel_states.process_search_active {
                        self.panel_states.process_tree_mode =
                            !self.panel_states.process_tree_mode;
                        self.panel_states.process_scroll_offset = 0;
                    }
                }
                // Collapse/expand in tree mode
                KeyCode::Right if self.panel_states.process_tree_mode => {
                    self.expand_collapse_tree(false);
                }
                KeyCode::Left if self.panel_states.process_tree_mode => {
                    self.expand_collapse_tree(true);
                }
                // Scrolling
                KeyCode::Up => {
                    self.panel_states.process_scroll_offset =
                        self.panel_states.process_scroll_offset.saturating_sub(1);
                }
                KeyCode::Down => {
                    self.panel_states.process_scroll_offset =
                        self.panel_states.process_scroll_offset.saturating_add(1);
                }
                KeyCode::PageUp => {
                    self.panel_states.process_scroll_offset =
                        self.panel_states.process_scroll_offset.saturating_sub(6);
                }
                KeyCode::PageDown => {
                    self.panel_states.process_scroll_offset =
                        self.panel_states.process_scroll_offset.saturating_add(6);
                }
                // Sort cycling
                KeyCode::Char('s') | KeyCode::Char('S') => {
                    if !self.panel_states.process_search_active {
                        let old = self.panel_states.process_sort_field;
                        self.panel_states.process_sort_field = old.next();
                        self.panel_states.process_scroll_offset = 0;
                    }
                }
                // Enter search mode
                KeyCode::Char('/') => {
                    self.panel_states.process_search_active = true;
                    self.panel_states.process_search.clear();
                }
                // Kill selected process
                KeyCode::Char('k') | KeyCode::Char('K') => {
                    if !self.panel_states.process_search_active {
                        self.kill_selected_process();
                    }
                }
                // When search is active, capture character input
                KeyCode::Char(c) if self.panel_states.process_search_active => {
                    self.panel_states.process_search.push(c);
                    self.panel_states.process_scroll_offset = 0;
                }
                KeyCode::Backspace if self.panel_states.process_search_active => {
                    self.panel_states.process_search.pop();
                    self.panel_states.process_scroll_offset = 0;
                }
                _ => {}
            },
            PanelId::Media => match key {
                KeyCode::Char(' ') => {
                    let _ = std::process::Command::new("playerctl")
                        .arg("play-pause").status();
                }
                KeyCode::Char('n') | KeyCode::Char('N') => {
                    let _ = std::process::Command::new("playerctl")
                        .arg("next").status();
                }
                KeyCode::Char('p') | KeyCode::Char('P') => {
                    let _ = std::process::Command::new("playerctl")
                        .arg("previous").status();
                }
                _ => {}
            },
            _ => {}
        }
    }

    /// Collapse or expand the process at the current scroll offset in tree mode.
    pub fn expand_collapse_tree(&mut self, collapse: bool) {
        if !self.panel_states.process_tree_mode {
            return;
        }
        let pid = processes::get_pid_at(
            self.panel_states.process_scroll_offset,
            self.panel_states.process_sort_field,
            self.panel_states.process_sort_asc,
            &self.panel_states.process_search,
            true,
            &self.panel_states.process_collapsed,
        );
        if let Some(pid) = pid {
            if collapse {
                self.panel_states.process_collapsed.insert(pid);
            } else {
                self.panel_states.process_collapsed.remove(&pid);
            }
        }
    }

    /// Kill the process at the current scroll offset in the processes panel.
    pub fn kill_selected_process(&mut self) {
        let pid = processes::get_pid_at(
            self.panel_states.process_scroll_offset,
            self.panel_states.process_sort_field,
            self.panel_states.process_sort_asc,
            &self.panel_states.process_search,
            self.panel_states.process_tree_mode,
            &self.panel_states.process_collapsed,
        );
        if let Some(pid) = pid {
            // Send SIGTERM
            let _ = process::Command::new("kill")
                .arg(pid.to_string())
                .status();
        }
    }

    /// Called on every refresh tick — advances animations, refreshes system data
    pub fn tick(&mut self) {
        self.tick_count = self.tick_count.wrapping_add(1);
    }

    pub fn render(&mut self, f: &mut Frame) {
        let area = f.area();

        let layouts = Layout::vertical([
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1),
        ]);
        let [title_bar, main_area, status_bar] = layouts.areas(area);

        // ── Collect summary for top bar ──
        let sum = collect_summary();

        // ── Title bar: summary header ──
        let theme_icon = if matches!(self.theme.bg, Color::Rgb(10, 10, 15)) {
            "🌙"
        } else {
            "☀"
        };

        let bat_str = sum.bat_pct.map_or(String::new(), |p| format!(" · 🔋{}%", p));

        let title_text = format!(
            " vanta {}  {} · {} · CPU {:.0}% · MEM {:.0}% · GPU {}% · ↓{} ↑{}{}     [T]heme [q]uit",
            theme_icon,
            sum.os.split_whitespace().next().unwrap_or("Linux"),
            sum.uptime,
            sum.cpu_pct,
            sum.mem_pct,
            sum.gpu_pct,
            sum.net_dl,
            sum.net_ul,
            bat_str,
        );
        let title_style = Style::default().fg(self.theme.dim).bg(self.theme.bg);
        f.render_widget(
            Paragraph::new(Line::from(Span::styled(&title_text, title_style)))
                .style(Style::default().bg(self.theme.bg)),
            title_bar,
        );

        // Main content — unified single pane
        overview::render(
            f,
            main_area,
            &self.theme,
            &self.config,
            self.tick_count,
            self.focused_panel,
            &self.panel_states,
        );

        // ── Status bar ──
        let focus_status = match self.focused_panel {
            Some(p) => format!("  Focus: {:?}", p),
            None => String::new(),
        };
        let process_status = match self.focused_panel {
            Some(PanelId::Processes) => {
                let sort = format!(" Sort:{}", self.panel_states.process_sort_field.label());
                if self.panel_states.process_search_active {
                    format!("{} | Search: [{}]", sort, self.panel_states.process_search)
                } else {
                    if self.panel_states.process_tree_mode {
                        format!("{} | Tree", sort)
                    } else {
                        sort
                    }
                }
            }
            _ => String::new(),
        };
        let status_style = Style::default().fg(self.theme.dim).bg(self.theme.surface);
        f.render_widget(
            Paragraph::new(Line::from(Span::styled(
                format!(
                    " vanta v0.1.0{}{}",
                    focus_status, process_status,
                ),
                status_style,
            )))
            .style(Style::default().bg(self.theme.surface)),
            status_bar,
        );
    }
}
