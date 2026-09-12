use std::collections::HashSet;
use std::time::{Duration, Instant};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::config::Config;
use crate::custom::CustomWidgetManager;
use crate::mode::DashboardMode;
use crate::monitors::{self, processes, Summary};
use crate::screens;
use crate::theme::Theme;
use crate::widgets::media::{self, Action};
use crate::widgets::music_viz;

/// Sort field for the processes panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortField {
    Cpu,
    Mem,
    Pid,
    Name,
}

impl SortField {
    pub fn label(&self) -> &'static str {
        match self {
            SortField::Cpu => "cpu",
            SortField::Mem => "mem",
            SortField::Pid => "pid",
            SortField::Name => "name",
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

/// Every focusable panel across all pages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanelId {
    System,
    Gauges,
    Cpu,
    Memory,
    Disk,
    Storage,
    Network,
    Gpu,
    Clock,
    Media,
    Visualizer,
    Processes,
    Status,
    Calendar,
    Matrix,
    Video,
    /// A user-defined custom widget at the given index in `CustomWidgetManager`.
    Custom(usize),
}

impl PanelId {
    /// Tab order for a page, honouring widget toggles.
    pub fn for_mode(mode: DashboardMode, cfg: &Config) -> Vec<PanelId> {
        let w = &cfg.widgets;
        let mut list: Vec<(PanelId, bool)> = match mode {
            DashboardMode::Dashboard => vec![
                (PanelId::System, true),
                (PanelId::Gauges, true),
                (PanelId::Cpu, w.cpu),
                (PanelId::Storage, w.disk),
                (PanelId::Clock, w.clock),
                (PanelId::Media, w.media),
                (PanelId::Visualizer, w.music_viz),
                (PanelId::Processes, w.processes),
                (PanelId::Status, true),
                (PanelId::Memory, w.memory),
                (PanelId::Network, w.network),
                (PanelId::Calendar, w.calendar),
            ],
            DashboardMode::Monitor => vec![
                (PanelId::Cpu, true),
                (PanelId::Memory, true),
                (PanelId::Disk, true),
                (PanelId::Network, true),
                (PanelId::Gpu, w.gpu),
                (PanelId::System, true),
                (PanelId::Processes, true),
            ],
            DashboardMode::Aesthetic => vec![
                (PanelId::Clock, true),
                (PanelId::Calendar, true),
                (PanelId::Matrix, w.matrix),
                (PanelId::Video, w.video),
                (PanelId::Visualizer, w.music_viz),
            ],
        };
        // Append enabled custom widgets (they only appear on the Dashboard page).
        if mode == DashboardMode::Dashboard {
            for (i, cw) in cfg.custom_widgets.iter().enumerate() {
                if cw.enabled {
                    list.push((PanelId::Custom(i), true));
                }
            }
        }
        list.into_iter()
            .filter(|(_, on)| *on)
            .map(|(p, _)| p)
            .collect()
    }

    pub fn label(&self) -> &'static str {
        match self {
            PanelId::System => "system",
            PanelId::Gauges => "gauges",
            PanelId::Cpu => "cpu",
            PanelId::Memory => "memory",
            PanelId::Disk => "disk",
            PanelId::Storage => "storage",
            PanelId::Network => "network",
            PanelId::Gpu => "gpu",
            PanelId::Clock => "clock",
            PanelId::Media => "media",
            PanelId::Visualizer => "visualizer",
            PanelId::Processes => "processes",
            PanelId::Status => "status",
            PanelId::Calendar => "calendar",
            PanelId::Matrix => "matrix",
            PanelId::Video => "donut",
            PanelId::Custom(_) => "custom",
        }
    }
}

/// Per-panel interactive state.
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
    pub process_collapsed: HashSet<u32>,
}

impl Default for PanelStates {
    fn default() -> Self {
        Self {
            calendar_month_offset: 0,
            process_scroll_offset: 0,
            process_sort_field: SortField::Cpu,
            process_sort_asc: false,
            process_search: String::new(),
            process_search_active: false,
            process_tree_mode: false,
            process_compact_cmd: true,
            process_selected_pid: None,
            process_collapsed: HashSet::new(),
        }
    }
}

pub struct App {
    pub running: bool,
    pub config: Config,
    pub theme: Theme,
    pub mode: DashboardMode,
    pub frame: u64,
    pub focused_panel: Option<PanelId>,
    pub panel_states: PanelStates,
    pub show_help: bool,
    /// A focused panel expanded to fill the page (Enter / Esc).
    pub zoomed: Option<PanelId>,
    pub summary: Summary,
    /// Short transient message shown in the status bar (theme changed, killed pid …).
    toast: Option<(String, Instant)>,
    /// First press of k/K arms this; the second press within the window fires.
    pending_signal: Option<(u32, &'static str, Instant)>,
    sampler_interval: std::sync::Arc<std::sync::atomic::AtomicU64>,
    /// Manager for all user-defined custom widgets.
    pub custom_widgets: CustomWidgetManager,
}

impl App {
    pub fn new(config: Config) -> Self {
        let theme = Theme::from_name(&config.ui.theme);
        let mode = DashboardMode::from_str(&config.ui.startup_mode);
        let sampler_interval = monitors::start(Duration::from_secs_f64(config.ui.refresh_rate));
        music_viz::set_style(&config.ui.visualizer);
        crate::widgets::gauge::set_style(&config.ui.gauge_style);
        crate::widgets::block_graph::set_style(&config.ui.graph_style);
        let custom_widgets = CustomWidgetManager::start_all(&config.custom_widgets);
        let mut app = Self {
            running: true,
            theme,
            config,
            mode,
            frame: 0,
            focused_panel: None,
            panel_states: PanelStates::default(),
            show_help: false,
            zoomed: None,
            summary: Summary::default(),
            toast: None,
            pending_signal: None,
            sampler_interval,
            custom_widgets,
        };
        if mode == DashboardMode::Monitor {
            app.focused_panel = Some(PanelId::Processes);
        }
        app
    }

    fn toast(&mut self, msg: impl Into<String>) {
        self.toast = Some((msg.into(), Instant::now()));
    }

    pub fn set_mode(&mut self, mode: DashboardMode) {
        if self.mode == mode {
            return;
        }
        self.mode = mode;
        self.zoomed = None;
        self.focused_panel = (mode == DashboardMode::Monitor).then_some(PanelId::Processes);
        self.config.ui.startup_mode = mode.as_str().to_string();
        self.config.save();
    }

    pub fn cycle_theme(&mut self) {
        let next = Theme::next_name(&self.config.ui.theme);
        self.config.ui.theme = next.to_string();
        self.theme = Theme::from_name(next);
        self.config.save();
        self.toast(format!("theme · {}", next));
    }

    fn cycle_focus(&mut self, forward: bool) {
        let panels = PanelId::for_mode(self.mode, &self.config);
        if panels.is_empty() {
            return;
        }
        let idx = self
            .focused_panel
            .and_then(|p| panels.iter().position(|&x| x == p));
        let next = match idx {
            Some(i) if forward => (i + 1) % panels.len(),
            Some(i) => (i + panels.len() - 1) % panels.len(),
            None if forward => 0,
            None => panels.len() - 1,
        };
        self.focused_panel = Some(panels[next]);
        if self.zoomed.is_some() {
            self.zoomed = self.focused_panel;
        }
    }

    fn adjust_refresh(&mut self, faster: bool) {
        let cur = self.config.ui.refresh_rate;
        let next = if faster { cur / 2.0 } else { cur * 2.0 }.clamp(0.1, 10.0);
        self.config.ui.refresh_rate = next;
        self.sampler_interval
            .store((next * 1000.0) as u64, std::sync::atomic::Ordering::Relaxed);
        self.config.save();
        self.toast(format!("refresh · {:.2}s", next));
    }

    // ── Input ─────────────────────────────────────────────────

    pub fn handle_key(&mut self, key: KeyEvent) {
        let ps = &mut self.panel_states;

        // Text entry captures everything first.
        if ps.process_search_active {
            match key.code {
                KeyCode::Esc => {
                    ps.process_search_active = false;
                    ps.process_search.clear();
                    ps.process_scroll_offset = 0;
                }
                KeyCode::Enter => ps.process_search_active = false,
                KeyCode::Backspace => {
                    ps.process_search.pop();
                    ps.process_scroll_offset = 0;
                }
                KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                    ps.process_search.push(c);
                    ps.process_scroll_offset = 0;
                }
                KeyCode::Up | KeyCode::Down | KeyCode::PageUp | KeyCode::PageDown => {
                    self.process_nav(key.code);
                    return;
                }
                _ => {}
            }
            self.sync_selected_pid();
            return;
        }

        if self.show_help {
            match key.code {
                KeyCode::Char('q') | KeyCode::Char('Q') => self.running = false,
                KeyCode::Char('T') => self.cycle_theme(),
                _ => self.show_help = false,
            }
            return;
        }

        match key.code {
            KeyCode::Char('q') | KeyCode::Char('Q') => self.running = false,
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.running = false
            }
            KeyCode::Char('?') | KeyCode::F(1) => self.show_help = true,
            KeyCode::Char('1') => self.set_mode(DashboardMode::Dashboard),
            KeyCode::Char('2') => self.set_mode(DashboardMode::Monitor),
            KeyCode::Char('3') => self.set_mode(DashboardMode::Aesthetic),
            KeyCode::Char('T') => self.cycle_theme(),
            KeyCode::Char('v') | KeyCode::Char('V') => {
                music_viz::cycle_style();
                self.config.ui.visualizer = music_viz::style_name().to_string();
                self.config.save();
                self.toast(format!("visualizer · {}", music_viz::style_name()));
            }
            KeyCode::Char('g') => {
                crate::widgets::gauge::cycle_style();
                self.config.ui.gauge_style = crate::widgets::gauge::style_name().to_string();
                self.config.save();
                self.toast(format!("gauges · {}", crate::widgets::gauge::style_name()));
            }
            KeyCode::Char('G') => {
                crate::widgets::block_graph::cycle_style();
                self.config.ui.graph_style = crate::widgets::block_graph::style_name().to_string();
                self.config.save();
                self.toast(format!("graphs · {}", crate::widgets::block_graph::style_name()));
            }
            KeyCode::Char('+') | KeyCode::Char('=') => self.adjust_refresh(true),
            KeyCode::Char('-') | KeyCode::Char('_') => self.adjust_refresh(false),
            KeyCode::Tab => self.cycle_focus(true),
            KeyCode::BackTab => self.cycle_focus(false),
            KeyCode::Enter => match (self.zoomed, self.focused_panel) {
                (Some(_), _) => self.zoomed = None,
                (None, Some(p)) => self.zoomed = Some(p),
                (None, None) => {}
            },
            KeyCode::Esc => {
                if self.zoomed.is_some() {
                    self.zoomed = None;
                } else {
                    self.focused_panel = None;
                }
            }

            // Media transport is global: it's the whole point of a dashboard.
            KeyCode::Char(' ') => media::control(Action::PlayPause),
            KeyCode::Char('n') | KeyCode::Char('N') => media::control(Action::Next),
            KeyCode::Char('p') | KeyCode::Char('P') => media::control(Action::Previous),
            KeyCode::Char('>') | KeyCode::Char('.') => media::control(Action::VolumeUp),
            KeyCode::Char('<') | KeyCode::Char(',') => media::control(Action::VolumeDown),

            _ => self.handle_panel_key(key.code),
        }
    }

    fn handle_panel_key(&mut self, key: KeyCode) {
        // Arrow keys with nothing focused grab the page's primary panel.
        if self.focused_panel.is_none()
            && matches!(
                key,
                KeyCode::Up
                    | KeyCode::Down
                    | KeyCode::Left
                    | KeyCode::Right
                    | KeyCode::PageUp
                    | KeyCode::PageDown
                    | KeyCode::Home
                    | KeyCode::End
            )
        {
            let panels = PanelId::for_mode(self.mode, &self.config);
            self.focused_panel = if panels.contains(&PanelId::Processes) {
                Some(PanelId::Processes)
            } else {
                panels.first().copied()
            };
        }
        // Process hotkeys work from anywhere on the Monitor page.
        let process_hotkey = matches!(
            key,
            KeyCode::Char('/')
                | KeyCode::Char('s')
                | KeyCode::Char('S')
                | KeyCode::Char('r')
                | KeyCode::Char('t')
                | KeyCode::Char('c')
                | KeyCode::Char('k')
                | KeyCode::Char('K')
        );
        if self.mode == DashboardMode::Monitor && process_hotkey {
            self.focused_panel = Some(PanelId::Processes);
        }

        match self.focused_panel {
            Some(PanelId::Calendar) => {
                let ps = &mut self.panel_states;
                match key {
                    KeyCode::Left | KeyCode::Char('h') => ps.calendar_month_offset -= 1,
                    KeyCode::Right | KeyCode::Char('l') => ps.calendar_month_offset += 1,
                    KeyCode::Up | KeyCode::Char('k') => ps.calendar_month_offset -= 12,
                    KeyCode::Down | KeyCode::Char('j') => ps.calendar_month_offset += 12,
                    KeyCode::Home | KeyCode::Char('t') => ps.calendar_month_offset = 0,
                    _ => {}
                }
            }
            Some(PanelId::Processes) => self.process_key(key),
            _ => {}
        }
    }

    fn process_nav(&mut self, key: KeyCode) {
        let ps = &mut self.panel_states;
        let page = 10;
        match key {
            KeyCode::Up => ps.process_scroll_offset = ps.process_scroll_offset.saturating_sub(1),
            KeyCode::Down => ps.process_scroll_offset = ps.process_scroll_offset.saturating_add(1),
            KeyCode::PageUp => {
                ps.process_scroll_offset = ps.process_scroll_offset.saturating_sub(page)
            }
            KeyCode::PageDown => {
                ps.process_scroll_offset = ps.process_scroll_offset.saturating_add(page)
            }
            KeyCode::Home => ps.process_scroll_offset = 0,
            KeyCode::End => ps.process_scroll_offset = processes::count().saturating_sub(1),
            _ => {}
        }
        self.sync_selected_pid();
    }

    fn process_key(&mut self, key: KeyCode) {
        match key {
            KeyCode::Up
            | KeyCode::Down
            | KeyCode::PageUp
            | KeyCode::PageDown
            | KeyCode::Home
            | KeyCode::End => self.process_nav(key),
            KeyCode::Char('t') => {
                let ps = &mut self.panel_states;
                ps.process_tree_mode = !ps.process_tree_mode;
                ps.process_scroll_offset = 0;
                self.sync_selected_pid();
            }
            KeyCode::Char('c') => {
                self.panel_states.process_compact_cmd = !self.panel_states.process_compact_cmd
            }
            KeyCode::Right if self.panel_states.process_tree_mode => self.set_collapsed(false),
            KeyCode::Left if self.panel_states.process_tree_mode => self.set_collapsed(true),
            KeyCode::Char('s') | KeyCode::Char('S') => {
                let ps = &mut self.panel_states;
                ps.process_sort_field = ps.process_sort_field.next();
                ps.process_scroll_offset = 0;
                self.sync_selected_pid();
            }
            KeyCode::Char('r') => {
                let ps = &mut self.panel_states;
                ps.process_sort_asc = !ps.process_sort_asc;
                ps.process_scroll_offset = 0;
                self.sync_selected_pid();
            }
            KeyCode::Char('/') => {
                let ps = &mut self.panel_states;
                ps.process_search_active = true;
                ps.process_search.clear();
                ps.process_scroll_offset = 0;
            }
            KeyCode::Char('k') => self.signal_selected("TERM"),
            KeyCode::Char('K') => self.signal_selected("KILL"),
            KeyCode::Char('x') | KeyCode::Char('X') => {
                self.pending_signal = None;
                self.toast("cancelled");
            }
            _ => {}
        }
    }

    fn selected_pid(&self) -> Option<u32> {
        let ps = &self.panel_states;
        processes::get_pid_at(
            ps.process_scroll_offset,
            ps.process_sort_field,
            ps.process_sort_asc,
            &ps.process_search,
            ps.process_tree_mode,
            &ps.process_collapsed,
        )
    }

    fn set_collapsed(&mut self, collapse: bool) {
        if let Some(pid) = self.selected_pid() {
            if collapse {
                self.panel_states.process_collapsed.insert(pid);
            } else {
                self.panel_states.process_collapsed.remove(&pid);
            }
            self.sync_selected_pid();
        }
    }

    fn signal_selected(&mut self, sig: &'static str) {
        let Some(pid) = self.selected_pid() else {
            return;
        };
        if pid == std::process::id() {
            self.toast("not killing myself — press q to quit");
            return;
        }
        // Two-step: the first press arms, the second (same pid + signal,
        // within 3s) fires. A mis-typed k on the wrong row is otherwise fatal.
        let armed = self
            .pending_signal
            .is_some_and(|(p, s, t)| p == pid && s == sig && t.elapsed() < Duration::from_secs(3));
        if !armed {
            let name = processes::name_of(pid).unwrap_or_default();
            self.pending_signal = Some((pid, sig, Instant::now()));
            self.toast(format!(
                "SIG{} → {} {}?  press {} again to confirm, x to cancel",
                sig,
                pid,
                name,
                if sig == "KILL" { "K" } else { "k" }
            ));
            return;
        }
        self.pending_signal = None;
        let ok = std::process::Command::new("kill")
            .args([&format!("-{}", sig), &pid.to_string()])
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        self.toast(if ok {
            format!("sent SIG{} to {}", sig, pid)
        } else {
            format!("failed to signal {} (permission?)", pid)
        });
    }

    pub fn sync_selected_pid(&mut self) {
        self.panel_states.process_selected_pid = self.selected_pid();
    }

    // ── Render ────────────────────────────────────────────────

    pub fn render(&mut self, f: &mut Frame) {
        self.frame = self.frame.wrapping_add(1);
        self.summary = monitors::summary();
        if self
            .toast
            .as_ref()
            .is_some_and(|(_, t)| t.elapsed() > Duration::from_secs(3))
        {
            self.toast = None;
        }

        // Advance custom widget history buffers before any rendering.
        self.custom_widgets.tick();

        let area = f.area();
        f.render_widget(
            ratatui::widgets::Block::default().style(Style::default().bg(self.theme.bg)),
            area,
        );
        let [title_bar, main, status_bar] = Layout::vertical([
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .areas(area);

        self.render_title(f, title_bar);

        match (self.zoomed, self.mode) {
            (Some(p), _) => screens::render_panel(f, main, self, p),
            (None, DashboardMode::Dashboard) => screens::dashboard::render(f, main, self),
            (None, DashboardMode::Monitor) => screens::monitor::render(f, main, self),
            (None, DashboardMode::Aesthetic) => screens::aesthetic::render(f, main, self),
        }

        self.render_status(f, status_bar);

        if self.show_help {
            screens::help::render(f, area, &self.theme, &self.config);
        }
    }

    fn render_title(&self, f: &mut Frame, area: Rect) {
        let t = &self.theme;
        let s = &self.summary;
        let base = Style::default().bg(t.bg);
        let dim = base.fg(t.dim);
        let sep = Span::styled("   ", dim);

        let mut worst = 0u8; // 0 ok, 1 warn, 2 crit
        let mut level = |pct: f64| {
            let l = if pct >= 90.0 {
                2
            } else if pct >= 75.0 {
                1
            } else {
                0
            };
            worst = worst.max(l);
            match l {
                2 => t.red,
                1 => t.yellow,
                _ => t.accent,
            }
        };

        let mut metrics: Vec<(&str, String, ratatui::style::Color)> = vec![
            (
                "cpu",
                format!("{:>3.0}%", s.cpu_pct),
                level(s.cpu_pct as f64),
            ),
            ("mem", format!("{:>3.0}%", s.mem_pct), level(s.mem_pct)),
        ];
        if let Some(g) = s.gpu_pct {
            metrics.push(("gpu", format!("{:>3.0}%", g), level(g)));
        }
        if let Some(d) = s.disk_pct {
            metrics.push(("disk", format!("{:>3.0}%", d), level(d)));
        }
        if let Some(c) = s.temp_c {
            metrics.push(("temp", format!("{:.0}°", c), level(c)));
        }
        metrics.push((
            "net",
            format!(
                "↓{} ↑{}",
                crate::widgets::meter::fmt_kbps(s.rx_kbps),
                crate::widgets::meter::fmt_kbps(s.tx_kbps)
            ),
            t.secondary,
        ));
        if let Some((p, charging)) = s.battery {
            let col = if charging || p > 20 {
                t.accent
            } else if p > 10 {
                worst = worst.max(1);
                t.yellow
            } else {
                worst = 2;
                t.red
            };
            metrics.push((
                "bat",
                format!("{}%{}", p, if charging { "⚡" } else { "" }),
                col,
            ));
        }
        metrics.push(("up", s.uptime.clone(), t.text));

        let dot = match worst {
            2 => t.red,
            1 => t.yellow,
            _ => t.green,
        };
        let mut left: Vec<Span> = vec![
            Span::styled(" vanta", base.fg(t.accent).add_modifier(Modifier::BOLD)),
            Span::styled(" ● ", base.fg(dot)),
        ];
        for (i, (k, v, c)) in metrics.iter().enumerate() {
            if i > 0 {
                left.push(sep.clone());
            }
            left.push(Span::styled(format!("{} ", k), dim));
            left.push(Span::styled(v.clone(), base.fg(*c).add_modifier(Modifier::BOLD)));
        }

        let mut right: Vec<Span> = Vec::new();
        for m in [
            DashboardMode::Dashboard,
            DashboardMode::Monitor,
            DashboardMode::Aesthetic,
        ] {
            let style = if m == self.mode {
                Style::default()
                    .fg(t.bg)
                    .bg(t.accent)
                    .add_modifier(Modifier::BOLD)
            } else {
                dim
            };
            right.push(Span::styled(
                format!(" {} {} ", m.hotkey(), m.label()),
                style,
            ));
            right.push(Span::styled(" ", base));
        }
        right.push(Span::styled("? help ", dim));

        let width = |v: &[Span]| -> usize { v.iter().map(|s| s.content.chars().count()).sum() };
        let (rw, avail) = (width(&right), area.width as usize);
        // Narrow terminal: shed metrics from the right (least important last)
        // until the page nav fits. Each metric is 3 spans (sep, key, value).
        while width(&left) + rw >= avail && left.len() > 2 + 3 {
            left.truncate(left.len() - 3);
        }
        let lw = width(&left);
        let mut spans = left;
        if lw + rw < avail {
            spans.push(Span::styled(" ".repeat(avail - lw - rw), base));
            spans.extend(right);
        } else if lw < avail {
            spans.push(Span::styled(" ".repeat(avail - lw), base));
        }
        f.render_widget(Paragraph::new(Line::from(spans)).style(base), area);
    }

    fn render_status(&self, f: &mut Frame, area: Rect) {
        let t = &self.theme;
        let base = Style::default().bg(t.surface);
        let key = base.fg(t.accent).add_modifier(Modifier::BOLD);
        let txt = base.fg(t.dim);

        let mut spans: Vec<Span> = vec![Span::styled(" ", base)];
        let mut hint = |k: &str, d: &str| {
            spans.push(Span::styled(k.to_string(), key));
            spans.push(Span::styled(format!(" {}   ", d), txt));
        };

        if let Some((msg, _)) = &self.toast {
            spans.push(Span::styled(
                format!("{}   ", msg),
                base.fg(t.text).add_modifier(Modifier::BOLD),
            ));
        } else if self.panel_states.process_search_active {
            hint("type", "to filter");
            hint("enter", "keep filter");
            hint("esc", "clear");
        } else {
            match self.focused_panel {
                Some(PanelId::Processes) => {
                    hint("↑↓", "select");
                    hint("/", "search");
                    hint("s", "sort");
                    hint("r", "reverse");
                    hint("t", "tree");
                    if self.panel_states.process_tree_mode {
                        hint("←→", "fold");
                    }
                    hint("c", "cmd");
                    hint("k", "term");
                    hint("K", "kill");
                }
                Some(PanelId::Calendar) => {
                    hint("←→", "month");
                    hint("↑↓", "year");
                    hint("home", "today");
                }
                _ => {
                    hint("tab", "focus");
                    if self.focused_panel.is_some() {
                        hint(
                            "enter",
                            if self.zoomed.is_some() {
                                "unzoom"
                            } else {
                                "zoom"
                            },
                        );
                    }
                    hint("space", "play/pause");
                    hint("n/p", "track");
                    hint("<>", "volume");
                    hint("v", "visualizer");
                    hint("T", "theme");
                    hint("+/-", "refresh");
                }
            }
            hint("q", "quit");
        }

        let mut right: Vec<Span> = Vec::new();
        if let Some(p) = self.focused_panel {
            right.push(Span::styled(format!("[{}] ", p.label()), base.fg(t.accent)));
        }
        if self.mode == DashboardMode::Monitor {
            let ps = &self.panel_states;
            right.push(Span::styled(
                format!(
                    "sort {}{} ",
                    ps.process_sort_field.label(),
                    if ps.process_sort_asc { "▴" } else { "▾" }
                ),
                txt,
            ));
        }
        right.push(Span::styled(
            format!(
                "{} · {}fps · {:.1}s · v{} ",
                self.config.ui.theme,
                self.config.ui.fps,
                self.config.ui.refresh_rate,
                env!("CARGO_PKG_VERSION")
            ),
            txt,
        ));

        let width = |v: &[Span]| -> usize { v.iter().map(|s| s.content.chars().count()).sum() };
        let (lw, rw, avail) = (width(&spans), width(&right), area.width as usize);
        if lw + rw < avail {
            spans.push(Span::styled(" ".repeat(avail - lw - rw), base));
            spans.extend(right);
        }
        f.render_widget(Paragraph::new(Line::from(spans)).style(base), area);
    }
}
