use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph};
use ratatui::Frame;

use crate::app::{self, PanelId, PanelStates, Summary};
use crate::config::Config;
use crate::monitors::{analytics, system_info};
use crate::widgets::{calendar, clock, media, music_viz, status, storage};

fn section_header(
    f: &mut Frame,
    area: Rect,
    label: &str,
    theme: &app::Theme,
    focused: bool,
) -> Rect {
    let bc = if focused { theme.accent } else { theme.surface };
    let tc = if focused { theme.accent } else { theme.dim };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(bc))
        .title(Span::styled(
            format!(" {} ", label),
            if focused {
                Style::default().fg(tc).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(tc)
            },
        ));
    let inner = block.inner(area);
    f.render_widget(block, area);
    inner
}

fn vcenter(area: Rect, height: u16) -> Rect {
    let h = height.min(area.height);
    let y = area.y + (area.height.saturating_sub(h)) / 2;
    Rect::new(area.x, y, area.width, h)
}

#[allow(clippy::too_many_arguments)]
pub fn render(
    f: &mut Frame,
    area: Rect,
    theme: &app::Theme,
    _config: &Config,
    _tick: u64,
    focused: Option<PanelId>,
    states: &PanelStates,
    sum: &Summary,
) {
    let cols = Layout::horizontal([
        Constraint::Ratio(1, 3),
        Constraint::Ratio(1, 3),
        Constraint::Ratio(1, 3),
    ])
    .split(area);

    // ── LEFT — all machine hardware / performance ───────────────────────────
    //   SYSTEM   (OS logo + neofetch kv)
    //   GAUGES   (arc rings: CPU / RAM / BAT)
    //   ANALYTICS (sparklines + plain value rows)
    //   STORAGE  (disk bars)
    {
        let rows = Layout::vertical([
            Constraint::Length(12), // SYSTEM   (10 inner + 2 border)
            Constraint::Length(10), // GAUGES   (8 inner + 2 border)
            Constraint::Min(6),     // ANALYTICS (stretches; min 6 so sparklines are visible)
            Constraint::Length(7),  // STORAGE  (5 inner + 2 border)
        ])
        .spacing(1)
        .split(cols[0]);

        // SYSTEM
        {
            let inner = section_header(
                f,
                rows[0],
                "SYSTEM",
                theme,
                focused == Some(PanelId::System),
            );
            let term = f.area();
            render_system(
                f,
                inner,
                theme,
                sum,
                term.width as usize,
                term.height as usize,
            );
        }

        // GAUGES
        {
            let inner = section_header(
                f,
                rows[1],
                "GAUGES",
                theme,
                focused == Some(PanelId::Memory),
            );
            let bat = sum.bat_pct.unwrap_or(0) as f64;
            let bat_col = if bat < 20.0 {
                theme.red
            } else if bat < 40.0 {
                theme.yellow
            } else {
                theme.green
            };
            let mem_col = if sum.mem_pct > 90.0 {
                theme.red
            } else if sum.mem_pct > 75.0 {
                theme.yellow
            } else {
                theme.accent
            };
            let cpu_col = if sum.cpu_pct > 90.0 {
                theme.red
            } else if sum.cpu_pct > 75.0 {
                theme.yellow
            } else {
                theme.accent
            };
            crate::widgets::gauge::render(
                f,
                inner,
                theme,
                &[
                    (
                        "CPU",
                        sum.cpu_pct as f64,
                        format!("{:.0}%", sum.cpu_pct),
                        cpu_col,
                    ),
                    ("RAM", sum.mem_pct, format!("{:.0}%", sum.mem_pct), mem_col),
                    ("BAT", bat, format!("{:.0}%", bat), bat_col),
                ],
            );
        }

        // ANALYTICS
        {
            let inner = section_header(
                f,
                rows[2],
                "ANALYTICS",
                theme,
                focused == Some(PanelId::Cpu),
            );
            analytics::render_compact(f, inner, theme, sum);
        }

        // STORAGE
        {
            let inner =
                section_header(f, rows[3], "STORAGE", theme, focused == Some(PanelId::Disk));
            storage::render(f, inner, theme);
        }
    }

    // ── CENTER — ambient / temporal ─────────────────────────────────────────
    //   CLOCK      (compact time + date)
    //   MEDIA      (now playing)
    //   VISUALIZER (audio bars — fixed height so it doesn't dominate)
    //   MATRIX     (fills remainder)
    {
        let rows = Layout::vertical([
            Constraint::Length(4),  // CLOCK
            Constraint::Length(6),  // MEDIA
            Constraint::Length(10), // VISUALIZER — fixed, not Min(0)
            Constraint::Min(0),     // MATRIX — gets the rest
        ])
        .spacing(1)
        .split(cols[1]);

        {
            let inner = section_header(f, rows[0], "CLOCK", theme, focused == Some(PanelId::Clock));
            clock::render(f, inner, theme);
        }
        {
            let inner = section_header(f, rows[1], "MEDIA", theme, focused == Some(PanelId::Media));
            media::render(f, inner, theme);
        }
        {
            let inner = section_header(
                f,
                rows[2],
                "VISUALIZER",
                theme,
                focused == Some(PanelId::Visualizer),
            );
            music_viz::render(f, inner, theme, _tick);
        }
        {
            let inner = section_header(
                f,
                rows[3],
                "TOP PROCESSES",
                theme,
                focused == Some(PanelId::Processes),
            );
            render_top_procs(f, inner, theme);
        }
    }

    // ── RIGHT — environment status + calendar ───────────────────────────────
    //   STATUS   (wifi / ip / pkgs / load / temps)
    //   CALENDAR (month grid, fills rest)
    {
        let rows = Layout::vertical([
            Constraint::Length(9),  // STATUS (7 rows + 2 border)
            Constraint::Length(10), // NETWORK GRAPH
            Constraint::Min(0),     // CALENDAR
        ])
        .spacing(1)
        .split(cols[2]);

        {
            let inner = section_header(f, rows[0], "STATUS", theme, false);
            status::render(f, inner, theme);
        }
        {
            let inner = section_header(
                f,
                rows[1],
                "NETWORK TRAFFIC",
                theme,
                focused == Some(PanelId::Network),
            );
            crate::monitors::network::render(f, inner, theme);
        }
        {
            let inner = section_header(
                f,
                rows[2],
                "CALENDAR",
                theme,
                focused == Some(PanelId::Calendar),
            );
            calendar::render(f, vcenter(inner, 9), theme, states.calendar_month_offset);
        }
    }
}

// ── System info panel: OS logo left, neofetch kv right ────────────────────────
fn render_system(
    f: &mut Frame,
    area: Rect,
    theme: &app::Theme,
    sum: &Summary,
    term_w: usize,
    term_h: usize,
) {
    if area.width < 24 || area.height < 3 {
        return;
    }

    let logo = os_logo(theme);
    let logo_w = 13u16.min(area.width.saturating_sub(12));

    // Logo left side
    let logo_area = Rect::new(area.x, area.y, logo_w, area.height);
    let lpad = area.height.saturating_sub(logo.len() as u16) / 2;
    let mut logo_lines: Vec<Line<'static>> = (0..lpad).map(|_| Line::from("")).collect();
    logo_lines.extend(logo);
    f.render_widget(Paragraph::new(logo_lines), logo_area);

    // KV right side
    let kv_area = Rect::new(
        area.x + logo_w + 1,
        area.y,
        area.width.saturating_sub(logo_w + 1),
        area.height,
    );

    let data = system_info::collect_neofetch(sum, term_w, term_h);
    let kv: &[(&str, &str)] = &[
        ("OS", &data.os),
        ("HOST", &data.host),
        ("KERN", &data.kernel),
        ("UP", &data.uptime),
        ("SH", &data.shell),
        ("RES", &data.resolution),
        ("CPU", &data.cpu),
        ("GPU", &data.gpu),
        ("MEM", &data.memory),
        ("BAT", &data.bat),
    ];

    let vpad = kv_area.height.saturating_sub(kv.len() as u16) / 2;
    let mut rows: Vec<Line<'static>> = (0..vpad).map(|_| Line::from("")).collect();
    let max_v = kv_area.width.saturating_sub(6) as usize;
    for (k, v) in kv {
        let v_str: String = v.chars().take(max_v).collect();
        rows.push(Line::from(vec![
            Span::styled(format!("{:>4} ", k), Style::default().fg(theme.dim)),
            Span::styled(
                v_str,
                Style::default().fg(theme.text).add_modifier(Modifier::BOLD),
            ),
        ]));
    }
    f.render_widget(Paragraph::new(rows), kv_area);
}

// ── Distro-detected OS logo ────────────────────────────────────────────────────
fn os_logo(theme: &app::Theme) -> Vec<Line<'static>> {
    let id = std::fs::read_to_string("/etc/os-release")
        .ok()
        .and_then(|s| {
            s.lines()
                .find(|l| l.starts_with("ID="))
                .map(|l| l.trim_start_matches("ID=").trim_matches('"').to_lowercase())
        })
        .unwrap_or_default();

    let c = theme.accent;

    match id.as_str() {
        "arch" => vec![
            Line::from(Span::styled("    /\\    ", Style::default().fg(c))),
            Line::from(Span::styled("   /  \\   ", Style::default().fg(c))),
            Line::from(Span::styled("  / /\\ \\  ", Style::default().fg(c))),
            Line::from(Span::styled(" / /  \\ \\ ", Style::default().fg(c))),
            Line::from(Span::styled("/_/ /\\ \\_\\", Style::default().fg(c))),
            Line::from(Span::styled("\\_\\/  \\/_/", Style::default().fg(c))),
        ],
        "ubuntu" => vec![
            Line::from(Span::styled(" ████████ ", Style::default().fg(c))),
            Line::from(Span::styled("██●  ●  ██", Style::default().fg(c))),
            Line::from(Span::styled("██   ●  ██", Style::default().fg(c))),
            Line::from(Span::styled(" ████████ ", Style::default().fg(c))),
        ],
        "fedora" => vec![
            Line::from(Span::styled("  ┌──────┐", Style::default().fg(c))),
            Line::from(Span::styled("  │  ╔═══╝", Style::default().fg(c))),
            Line::from(Span::styled("══╪══╣    ", Style::default().fg(c))),
            Line::from(Span::styled("  │  ╚═══╗", Style::default().fg(c))),
            Line::from(Span::styled("  └──────┘", Style::default().fg(c))),
        ],
        "debian" => vec![
            Line::from(Span::styled("  ╭────╮  ", Style::default().fg(c))),
            Line::from(Span::styled(" ╭╯  ╭─╯  ", Style::default().fg(c))),
            Line::from(Span::styled("╰╮  ╰─╮   ", Style::default().fg(c))),
            Line::from(Span::styled(" ╰────╯   ", Style::default().fg(c))),
        ],
        _ => vec![
            Line::from(Span::styled("╭────────╮", Style::default().fg(c))),
            Line::from(Span::styled("│  Linux │", Style::default().fg(c))),
            Line::from(Span::styled("╰────────╯", Style::default().fg(c))),
        ],
    }
}

// ── Top Processes summary widget ──────────────────────────────────────────────
fn render_top_procs(f: &mut Frame, area: Rect, theme: &app::Theme) {
    if area.height < 2 || area.width < 18 {
        return;
    }

    let sys = crate::app::SYS.lock().unwrap();
    let mut procs: Vec<_> = sys.processes().values().collect();
    procs.sort_by(|a, b| {
        b.cpu_usage()
            .partial_cmp(&a.cpu_usage())
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let fmt_mem = |bytes: u64| {
        if bytes >= 1024 * 1024 * 1024 {
            format!("{:.1}G", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
        } else if bytes >= 1024 * 1024 {
            format!("{:.0}M", bytes as f64 / (1024.0 * 1024.0))
        } else {
            format!("{}K", bytes / 1024)
        }
    };

    let mut lines: Vec<Line> = Vec::new();

    // Column widths
    let name_w = (area.width as usize).saturating_sub(21).max(6);
    lines.push(Line::from(vec![
        Span::styled(format!("{:<6} ", "PID"), Style::default().fg(theme.dim)),
        Span::styled(
            format!("{:<width$} ", "NAME", width = name_w),
            Style::default().fg(theme.dim),
        ),
        Span::styled(format!("{:>6} ", "CPU%"), Style::default().fg(theme.dim)),
        Span::styled(format!("{:>6}", "MEM"), Style::default().fg(theme.dim)),
    ]));

    let max_rows = (area.height as usize).saturating_sub(1);
    for p in procs.iter().take(max_rows) {
        let cpu = p.cpu_usage();
        let cpu_col = if cpu > 80.0 {
            theme.red
        } else if cpu > 40.0 {
            theme.yellow
        } else {
            theme.text
        };
        let name_str: String = p.name().to_string_lossy().chars().take(name_w).collect();

        lines.push(Line::from(vec![
            Span::styled(
                format!("{:<6} ", p.pid().as_u32()),
                Style::default().fg(theme.dim),
            ),
            Span::styled(
                format!("{:<width$} ", name_str, width = name_w),
                Style::default().fg(theme.text).add_modifier(Modifier::BOLD),
            ),
            Span::styled(format!("{:>5.1}% ", cpu), Style::default().fg(cpu_col)),
            Span::styled(
                format!("{:>6}", fmt_mem(p.memory())),
                Style::default().fg(theme.secondary),
            ),
        ]));
    }

    f.render_widget(Paragraph::new(lines), area);
}
