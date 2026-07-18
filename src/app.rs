use std::sync::{LazyLock, Mutex};

use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

pub static SYS: LazyLock<Mutex<sysinfo::System>> = LazyLock::new(|| {
    let mut s = sysinfo::System::new_all();
    s.refresh_all();
    Mutex::new(s)
});

use std::collections::HashSet;
use std::fs;
use std::process;

use crate::config::Config;
use crate::layout;
use crate::mode::DashboardMode;
use crate::monitors::processes;
use crate::screens::overview;
use crate::widgets::widget;

/// Summary metrics collected once per render for the top bar
struct Summary {
    cpu_pct: f32,
    mem_pct: f64,
    gpu_pct: u64,
    disk_pct: f64,
    net_dl: String,
    net_ul: String,
    bat_pct: Option<u8>,
    uptime: String,
    os: String,
}

fn collect_summary() -> Summary {
    let mut sys = SYS.lock().unwrap();
    sys.refresh_cpu_all();
    let cpu_pct = sys.global_cpu_usage();

    // Memory
    sys.refresh_memory();
    let mem_pct = if sys.total_memory() > 0 {
        (sys.used_memory() as f64 / sys.total_memory() as f64) * 100.0
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

    // Disk usage (root filesystem via df)
    let disk_pct = std::process::Command::new("df")
        .args(["-h", "--output=pcent", "/"])
        .output()
        .ok()
        .and_then(|o| {
            let s = String::from_utf8_lossy(&o.stdout);
            s.lines().nth(1)?.trim().trim_end_matches('%').parse::<f64>().ok()
        })
        .unwrap_or(0.0);

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
        disk_pct,
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
    Rss,
}

impl SortField {
    pub fn label(&self) -> &'static str {
        match self {
            SortField::Cpu => "CPU",
            SortField::Mem => "MEM",
            SortField::Pid => "PID",
            SortField::Name => "NAME",
            SortField::Rss => "RSS",
        }
    }
    pub fn next(&self) -> Self {
        match self {
            SortField::Cpu => SortField::Mem,
            SortField::Mem => SortField::Pid,
            SortField::Pid => SortField::Name,
            SortField::Name => SortField::Rss,
            SortField::Rss => SortField::Cpu,
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
    Profile,
    Video,
}

impl PanelId {
    /// Ordered list for Tab-cycle, excluding disabled widgets
    pub fn all(config: &Config) -> Vec<PanelId> {
        let mut v = Vec::with_capacity(12);
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
        if config.widgets.profile { v.push(PanelId::Profile); }
        if config.widgets.video { v.push(PanelId::Video); }
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
    pub process_compact_cmd: bool,
    pub process_selected_pid: Option<u32>,
    pub process_show_detail: bool,
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
            process_compact_cmd: true,
            process_selected_pid: None,
            process_show_detail: false,
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
    pub fn from_name(name: &str) -> Self {
        match name {
            "dark" => Self::dark(),
            "light" => Self::light(),
            "dracula" => Self::dracula(),
            "solarized-light" => Self::solarized_light(),
            _ => Self::dark(),
        }
    }

    pub fn dracula() -> Self {
        Self {
            bg: Color::Rgb(40, 42, 54),
            accent: Color::Rgb(255, 85, 85), // red accent
            secondary: Color::Rgb(98, 114, 164),
            surface: Color::Rgb(68, 71, 90),
            text: Color::Rgb(248, 248, 242),
            dim: Color::Rgb(98, 114, 164),
            green: Color::Rgb(80, 250, 123),
            yellow: Color::Rgb(255, 203, 107),
            red: Color::Rgb(255, 121, 198),
            focus: Color::Rgb(80, 250, 123),
        }
    }

    pub fn solarized_light() -> Self {
        Self {
            bg: Color::Rgb(253, 246, 227),
            accent: Color::Rgb(38, 139, 210),
            secondary: Color::Rgb(108, 113, 196),
            surface: Color::Rgb(238, 232, 213),
            text: Color::Rgb(101, 123, 131),
            dim: Color::Rgb(147, 161, 161),
            green: Color::Rgb(133, 153, 0),
            yellow: Color::Rgb(181, 137, 0),
            red: Color::Rgb(220, 50, 47),
            focus: Color::Rgb(38, 139, 210),
        }
    }

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
    pub mode: DashboardMode,
    pub tick_count: u64,
    pub focused_panel: Option<PanelId>,
    pub panel_states: PanelStates,
}

impl App {
    pub fn new(config: Config) -> Self {
        // Initialize theme based on saved config theme name
        let init_theme = Theme::from_name(&config.ui.theme);
        // Initialize mode from saved startup mode, fall back to Overview
        let init_mode = DashboardMode::from_str(&config.ui.startup_mode);
        let mut app = Self {
            running: true,
            theme: init_theme,
            config,
            mode: init_mode,
            tick_count: 0,
            focused_panel: None,
            panel_states: PanelStates::new(),
        };
        // Ensure mode is persisted on first run
        app.persist_mode();
        app
    }

    /// Persist the current mode to config and save.
    pub fn persist_mode(&mut self) {
        self.config.ui.startup_mode = self.mode.as_str().to_string();
        self.config.save();
    }

    /// Switch mode and persist to config.
    pub fn set_mode(&mut self, mode: DashboardMode) {
        self.mode = mode;
        self.focused_panel = None;
        self.persist_mode();
    }

    /// Cycle through available themes (used by UI hotkey).
    pub fn toggle_theme(&mut self) {
        const THEME_ORDER: [&str; 4] = ["dark", "light", "dracula", "solarized-light"];
        let current = &self.config.ui.theme;
        let idx = THEME_ORDER.iter().position(|&n| n == current).unwrap_or(0);
        let next_idx = (idx + 1) % THEME_ORDER.len();
        let next_name = THEME_ORDER[next_idx];
        self.set_theme(next_name);
    }

    /// Set theme by name (validates and persists).
    pub fn set_theme(&mut self, name: &str) {
        const THEME_ORDER: [&str; 4] = ["dark", "light", "dracula", "solarized-light"];
        if !THEME_ORDER.contains(&name) {
            // fallback to dark if unknown
            self.config.ui.theme = "dark".to_string();
            self.theme = Theme::dark();
        } else {
            self.config.ui.theme = name.to_string();
            self.theme = Theme::from_name(name);
        }
        // Persist the chosen theme
        self.config.save();
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
                        self.panel_states.process_tree_mode = !self.panel_states.process_tree_mode;
                        self.panel_states.process_scroll_offset = 0;
                    }
                }
                // Compact/full command toggle
                KeyCode::Char('c') | KeyCode::Char('C') => {
                    self.panel_states.process_compact_cmd =
                        !self.panel_states.process_compact_cmd;
                }
                // Info/detail toggle
                KeyCode::Char('i') | KeyCode::Char('I') => {
                    self.panel_states.process_show_detail =
                        !self.panel_states.process_show_detail;
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
        SYS.lock().unwrap().refresh_all();
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

        // ── Title bar: health summary bar with semantic colors ──
        let dim = self.theme.dim;
        let bg = self.theme.bg;
        let green = self.theme.green;
        let yellow = self.theme.yellow;
        let red = self.theme.red;
        let secondary = self.theme.secondary;
        let _accent = self.theme.accent;

        // Semantic coloring helpers
        let cpu_col = if sum.cpu_pct > 80.0 { red } else if sum.cpu_pct > 50.0 { yellow } else { green };
        let mem_col = if sum.mem_pct > 90.0 { red } else if sum.mem_pct > 70.0 { yellow } else { green };
        let gpu_col = if sum.gpu_pct > 80 { red } else if sum.gpu_pct > 50 { yellow } else { green };
        let disk_col = if sum.disk_pct > 90.0 { red } else if sum.disk_pct > 70.0 { yellow } else { green };
        let bat_col = match sum.bat_pct {
            Some(p) if p < 10 => red,
            Some(p) if p < 20 => yellow,
            _ => green,
        };

        let os_name = sum.os.split_whitespace().next().unwrap_or("Linux");

        let sep = Span::styled(" │", Style::default().fg(dim).bg(bg));

        let mut bar = Vec::with_capacity(12);

        // Group 1: OS + uptime
        bar.push(Span::styled(format!(" {} {} ", os_name, sum.uptime), Style::default().fg(dim).bg(bg)));
        bar.push(sep.clone());

        // Group 2: Health stats (semantic colours)
        bar.push(Span::styled(format!("CPU {:.0}%", sum.cpu_pct), Style::default().fg(cpu_col).bg(bg)));
        bar.push(Span::styled(format!(" MEM {:.0}%", sum.mem_pct), Style::default().fg(mem_col).bg(bg)));
        bar.push(Span::styled(format!(" DISK {:.0}%", sum.disk_pct), Style::default().fg(disk_col).bg(bg)));
        bar.push(Span::styled(format!(" GPU {}%", sum.gpu_pct), Style::default().fg(gpu_col).bg(bg)));
        bar.push(sep.clone());

        // Group 3: Network
        bar.push(Span::styled(format!(" ↓{}", sum.net_dl), Style::default().fg(secondary).bg(bg)));
        bar.push(Span::styled(format!(" ↑{}", sum.net_ul), Style::default().fg(secondary).bg(bg)));

        // Group 4: Battery
        if let Some(p) = sum.bat_pct {
            bar.push(Span::styled(format!(" {}%", p), Style::default().fg(bat_col).bg(bg)));
        }
        bar.push(sep.clone());

        // Nav
        let nav_entries = ["1O", "2M", "3P", "4D", "5A", "6S", "T\u{2191}", "Q\u{2190}"];
        let nav_str = nav_entries.join(" ");
        let nav_display = format!(" {}", nav_str);
        let nav_len = nav_display.len();

        // Push stats to the left, nav to the right
        let stats_len: usize = bar.iter().map(|s| s.content.len()).sum();
        let avail = title_bar.width as usize;
        let gap = if stats_len + nav_len + 2 < avail {
            avail.saturating_sub(stats_len + nav_len)
        } else {
            2
        };

        bar.push(Span::styled(" ".repeat(gap), Style::default().bg(bg)));
        bar.push(Span::styled(nav_display, Style::default().fg(dim).bg(bg)));
        // Fill remainder
        let used: usize = bar.iter().map(|s| s.content.len()).sum();
        if used < avail {
            bar.push(Span::styled(" ".repeat(avail - used), Style::default().bg(bg)));
        }

        f.render_widget(
            Paragraph::new(Line::from(bar))
                .style(Style::default().bg(bg)),
            title_bar,
        );

        // Main content — dispatch by mode
        match self.mode {
            DashboardMode::Overview => {
                overview::render(
                    f,
                    main_area,
                    &self.theme,
                    &self.config,
                    self.tick_count,
                    self.focused_panel,
                    &self.panel_states,
                );
            }
            DashboardMode::Monitor
            | DashboardMode::Aesthetic => {
                // Black background
                f.render_widget(
                    Paragraph::new("").style(Style::default().bg(Color::Rgb(0, 0, 0))),
                    main_area,
                );

                let placements = layout::layout_for_mode(self.mode, main_area, &self.config);
                let widgets = widget::widgets_for_mode(self.mode);

                for placement in placements {
                    if let Some(w) = widgets.iter().find(|w| w.id() == placement.id) {
                        w.render(
                            f,
                            placement.area,
                            &self.theme,
                            &self.config,
                            &self.panel_states,
                            self.tick_count,
                        );
                    }
                }
            }
        }

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
                    " vanta v0.1.0 [{}]{}{}",
                    self.mode.label(),
                    focus_status, process_status,
                ),
                status_style,
            )))
            .style(Style::default().bg(self.theme.surface)),
            status_bar,
        );
    }
}
