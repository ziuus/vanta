use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph};
use ratatui::Frame;

use crate::app::{self, PanelId, PanelStates, Summary};
use crate::config::Config;
use crate::monitors::{analytics, processes, system_info};
use crate::widgets::{calendar, clock, media, profile};

fn section_header(
    f: &mut Frame,
    area: Rect,
    label: &str,
    theme: &app::Theme,
    focused: bool,
) -> Rect {
    if label.is_empty() {
        return area;
    }
    let title_color = if focused { theme.accent } else { theme.dim };
    let border_color = if focused { theme.accent } else { theme.dim };

    let b_type = if focused {
        BorderType::Thick
    } else {
        BorderType::Rounded
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(b_type)
        .border_style(Style::default().fg(border_color))
        .title(Span::styled(
            format!(" {} ", label),
            Style::default()
                .fg(title_color)
                .add_modifier(ratatui::style::Modifier::BOLD),
        ));

    let inner = block.inner(area);
    f.render_widget(block, area);
    inner
}

/// Vertically center a `height`-tall block inside `area`.
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
    let rows = Layout::vertical([
        Constraint::Length(10), // SYSTEM neofetch hero
        Constraint::Length(7),  // CLOCK | ANALYTICS | MEDIA
        Constraint::Min(0),     // PROFILE | CALENDAR | PROCESSES
    ])
    .spacing(1)
    .split(area);
    if rows.len() < 3 {
        return;
    }

    // ── SYSTEM (neofetch) ──
    let inner = section_header(
        f,
        rows[0],
        "󰒋 SYSTEM",
        theme,
        focused == Some(PanelId::System),
    );
    let term = f.area();
    render_neofetch(
        f,
        inner,
        theme,
        sum,
        term.width as usize,
        term.height as usize,
    );

    // ── Middle: CLOCK | ANALYTICS | MEDIA ──
    let mid = Layout::horizontal([
        Constraint::Length(36),
        Constraint::Fill(1),
        Constraint::Length(36),
    ])
    .spacing(1)
    .split(rows[1]);
    if mid.len() < 3 {
        return;
    }

    let inner = section_header(f, mid[0], "󰥔 CLOCK", theme, focused == Some(PanelId::Clock));
    clock::render(f, vcenter(inner, 3), theme);

    let inner = section_header(
        f,
        mid[1],
        "󰋼 ANALYTICS",
        theme,
        focused == Some(PanelId::Cpu),
    );
    analytics::render(f, inner, theme, sum);

    let inner = section_header(f, mid[2], "󰝚 MEDIA", theme, focused == Some(PanelId::Media));
    media::render(f, inner, theme);

    // ── Bottom: PROFILE | CALENDAR | PROCESSES ──
    let bot = Layout::horizontal([
        Constraint::Length(60),
        Constraint::Length(26),
        Constraint::Fill(1),
    ])
    .spacing(1)
    .split(rows[2]);
    if bot.len() < 3 {
        return;
    }

    let inner = section_header(
        f,
        bot[0],
        " PROFILE",
        theme,
        focused == Some(PanelId::Profile),
    );
    profile::render(f, inner, theme);

    let inner = section_header(
        f,
        bot[1],
        "󰃭 CALENDAR",
        theme,
        focused == Some(PanelId::Calendar),
    );
    calendar::render(f, vcenter(inner, 9), theme, states.calendar_month_offset);

    let inner = section_header(
        f,
        bot[2],
        "󰒓 PROCESSES",
        theme,
        focused == Some(PanelId::Processes),
    );
    render_process_preview(f, inner, theme);
}

/// Neofetch-style hero: ASCII logo on the left, key/value rows on the right.
fn render_neofetch(
    f: &mut Frame,
    area: Rect,
    theme: &app::Theme,
    sum: &Summary,
    term_w: usize,
    term_h: usize,
) {
    if area.width < 60 || area.height < 3 {
        return;
    }
    let data = system_info::collect_neofetch(sum, term_w, term_h);

    let logo_w = 30u16.min(area.width / 4);
    let logo_area = Rect::new(area.x, area.y, logo_w, area.height);
    let art = profile::ascii_art(logo_area.width, area.height);
    let mut logo_lines: Vec<Line<'static>> = Vec::new();
    for _ in 0..area.height.saturating_sub(art.len() as u16) / 2 {
        logo_lines.push(Line::from(""));
    }
    logo_lines.extend(art);
    f.render_widget(
        Paragraph::new(logo_lines).alignment(ratatui::layout::Alignment::Left),
        logo_area,
    );

    let val_area = Rect::new(area.x + logo_w, area.y, area.width - logo_w, area.height);
    let halves = Layout::horizontal([Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)])
        .spacing(2)
        .split(val_area);

    let left = [
        kv_line("OS", &data.os, theme),
        kv_line("HOST", &data.host, theme),
        kv_line("KERNEL", &data.kernel, theme),
        kv_line("UPTIME", &data.uptime, theme),
        kv_line("SHELL", &data.shell, theme),
        kv_line("RESOLUTION", &data.resolution, theme),
    ];
    let right = [
        kv_line("CPU", &data.cpu, theme),
        kv_line("GPU", &data.gpu, theme),
        kv_line("MEMORY", &data.memory, theme),
        kv_line("BATTERY", &data.bat, theme),
        Line::from(""),
        Line::from(""),
    ];

    let pad = area.height.saturating_sub(6) / 2;
    let mut l: Vec<Line<'static>> = Vec::new();
    let mut r: Vec<Line<'static>> = Vec::new();
    for _ in 0..pad {
        l.push(Line::from(""));
        r.push(Line::from(""));
    }
    l.extend(left);
    r.extend(right);

    f.render_widget(
        Paragraph::new(l).alignment(ratatui::layout::Alignment::Right),
        halves[0],
    );
    f.render_widget(
        Paragraph::new(r).alignment(ratatui::layout::Alignment::Left),
        halves[1],
    );
}

fn kv_line(key: &str, value: &str, theme: &app::Theme) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("{:>9}  ", key), Style::default().fg(theme.dim)),
        Span::styled(
            value.to_string(),
            Style::default().fg(theme.text).add_modifier(Modifier::BOLD),
        ),
    ])
}

/// Compact top-by-memory process list for the Overview preview.
fn render_process_preview(f: &mut Frame, area: Rect, theme: &app::Theme) {
    if area.height < 2 {
        return;
    }
    let rows = processes::top_by_mem(area.height.saturating_sub(1) as usize);
    if rows.is_empty() {
        f.render_widget(
            Paragraph::new(Line::from(Span::styled(
                " (no process data)",
                Style::default().fg(theme.dim),
            ))),
            area,
        );
        return;
    }

    let w = area.width as usize;
    let name_w = w.saturating_sub(25).max(8);

    let mut lines: Vec<Line<'static>> = Vec::new();

    let mut hdr: Vec<Span<'static>> = Vec::new();
    hdr.push(Span::styled(" PID  ", Style::default().fg(theme.dim)));
    hdr.push(Span::styled(
        format!("{:<1$}", "NAME", name_w),
        Style::default().fg(theme.dim),
    ));
    hdr.push(Span::styled(
        format!("  {:>6}  {:>5}", "MEM", "CPU"),
        Style::default().fg(theme.dim),
    ));
    lines.push(Line::from(hdr));

    for (pid, name, mem_kb, cpu) in rows {
        let mut name_disp = name.clone();
        if name_disp.chars().count() > name_w {
            let mut t: String = name_disp.chars().take(name_w.saturating_sub(1)).collect();
            t.push('…');
            name_disp = t;
        }

        let mem_s = if mem_kb >= 1024 * 1024 {
            format!("{:.1}G", mem_kb as f64 / (1024.0 * 1024.0))
        } else {
            format!("{:.0}M", mem_kb as f64 / 1024.0)
        };
        let cpu_col = if cpu > 10.0 { theme.accent } else { theme.dim };

        let mut spans: Vec<Span<'static>> = Vec::new();
        spans.push(Span::styled(
            format!(" {:>5} ", pid),
            Style::default().fg(theme.dim),
        ));
        spans.push(Span::styled(
            format!("{:<1$}", name_disp, name_w),
            Style::default().fg(theme.text),
        ));
        spans.push(Span::styled(
            format!("  {:>6}  ", mem_s),
            Style::default().fg(theme.secondary),
        ));
        spans.push(Span::styled(
            format!("{:>4.1}%", cpu),
            Style::default().fg(cpu_col).add_modifier(Modifier::BOLD),
        ));
        lines.push(Line::from(spans));
    }

    f.render_widget(
        Paragraph::new(lines).style(Style::default().bg(theme.bg)),
        area,
    );
}
