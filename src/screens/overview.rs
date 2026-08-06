use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph};
use ratatui::Frame;

use crate::app::{self, PanelId, PanelStates, Summary};
use crate::config::Config;
use crate::monitors::{analytics, system_info};
use crate::widgets::{
    calendar, clock, cores, gauge, heatmap, media, music_viz, profile, status, storage,
};

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

    let b_type = BorderType::Rounded;

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(b_type)
        .border_style(Style::default().fg(border_color))
        .title(Span::styled(
            format!(" {} ", label),
            if focused {
                Style::default()
                    .fg(title_color)
                    .add_modifier(ratatui::style::Modifier::BOLD)
            } else {
                Style::default().fg(title_color)
            },
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
        Constraint::Length(6),  // SYSTEM neofetch hero
        Constraint::Length(12), // CLOCK | MEDIA | ANALYTICS
        Constraint::Length(10), // STATUS | STORAGE | GAUGES | VISUALIZER
        Constraint::Min(0),     // PROFILE | CALENDAR | MATRIX | COWSAY
    ])
    .spacing(1)
    .split(area);
    if rows.len() < 4 {
        return;
    }

    // ── SYSTEM (neofetch) ──
    let inner = section_header(
        f,
        rows[0],
        "󰓋 SYSTEM",
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

    // ── Middle: CLOCK | MEDIA | ANALYTICS ── (ratios so it spans any width)
    let mid = Layout::horizontal([
        Constraint::Ratio(1, 4),
        Constraint::Ratio(2, 4),
        Constraint::Ratio(1, 4),
    ])
    .spacing(1)
    .split(rows[1]);
    if mid.len() < 3 {
        return;
    }

    let inner = section_header(f, mid[0], "󰥔 CLOCK", theme, focused == Some(PanelId::Clock));
    clock::render(f, inner, theme);

    let inner = section_header(f, mid[1], "󰝚 MEDIA", theme, focused == Some(PanelId::Media));
    media::render(f, inner, theme);

    let inner = section_header(
        f,
        mid[2],
        "󰋼 ANALYTICS",
        theme,
        focused == Some(PanelId::Cpu),
    );
    analytics::render_compact(f, inner, theme, sum);

    // ── STATUS | STORAGE | VISUALIZER ──
    let row2 = Layout::horizontal([
        Constraint::Ratio(3, 16),
        Constraint::Ratio(3, 16),
        Constraint::Ratio(4, 16),
        Constraint::Ratio(6, 16),
    ])
    .spacing(1)
    .split(rows[2]);
    if row2.len() < 4 {
        return;
    }

    let inner = section_header(
        f,
        row2[0],
        "󰈀 STATUS",
        theme,
        focused == Some(PanelId::Network),
    );
    status::render(f, inner, theme);

    let inner = section_header(
        f,
        row2[1],
        "󰋊 STORAGE",
        theme,
        focused == Some(PanelId::Disk),
    );
    storage::render(f, inner, theme);

    let inner = section_header(
        f,
        row2[2],
        "󰈈 GAUGES",
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
    gauge::render(
        f,
        inner,
        theme,
        &[
            ("BAT", bat, format!("{:.0}%", bat), bat_col),
            ("RAM", sum.mem_pct, format!("{:.0}%", sum.mem_pct), mem_col),
            (
                "CPU",
                sum.cpu_pct as f64,
                format!("{:.0}%", sum.cpu_pct),
                cpu_col,
            ),
        ],
    );

    let inner = section_header(
        f,
        row2[3],
        "󰝚 VISUALIZER",
        theme,
        focused == Some(PanelId::Visualizer),
    );
    music_viz::render(f, inner, theme, _tick);

    // ── Bottom: PROFILE | CALENDAR | CORES | CPU HEAT | MATRIX ──
    let bot = Layout::horizontal([
        Constraint::Ratio(1, 5),
        Constraint::Ratio(1, 5),
        Constraint::Ratio(1, 5),
        Constraint::Ratio(1, 5),
        Constraint::Ratio(1, 5),
    ])
    .spacing(1)
    .split(rows[3]);
    if bot.len() < 5 {
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
        "\u{f0ee0} CORES",
        theme,
        focused == Some(PanelId::Cpu),
    );
    cores::render(f, inner, theme);

    let inner = section_header(
        f,
        bot[3],
        "\u{f0ee0} CPU HEAT",
        theme,
        focused == Some(PanelId::Cpu),
    );
    heatmap::render(f, inner, theme);

    let inner = section_header(
        f,
        bot[4],
        "󰘦 MATRIX",
        theme,
        false, // Cmatrix doesn't need focus yet
    );
    crate::widgets::cmatrix::render(f, inner, _tick, theme);
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

    // Tip bubble lives in the hero's right margin, which the kv columns leave
    // empty on wide terminals.
    if area.width >= 150 {
        let tip_w = area.width / 4;
        let tip_area = Rect::new(area.x + area.width - tip_w, area.y, tip_w, area.height);
        crate::widgets::cowsay::render(f, tip_area, theme);
    }

    let logo_w = 48u16.min(area.width / 3);
    let logo_area = Rect::new(area.x, area.y, logo_w, area.height);
    let art = profile::wordmark(theme);
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
