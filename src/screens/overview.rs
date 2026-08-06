use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph};
use ratatui::Frame;

use crate::app::{self, PanelId, PanelStates, Summary};
use crate::config::Config;
use crate::monitors::{analytics, system_info};
use crate::widgets::{clock, cores, heatmap, music_viz, profile, status, storage};

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
    _states: &PanelStates,
    sum: &Summary,
) {
    let rows = Layout::vertical([
        Constraint::Ratio(1, 3), // PRIMARY: STATUS | CPU HEAT
        Constraint::Ratio(1, 3), // SECONDARY: CORES | STORAGE | ANALYTICS
        Constraint::Ratio(1, 3), // TERTIARY: CLOCK | VISUALIZER
    ])
    .spacing(1)
    .split(area);
    if rows.len() < 3 {
        return;
    }

    // ── Row 1: PRIMARY ──
    let row1 = Layout::horizontal([
        Constraint::Ratio(1, 3), // STATUS (4 cols)
        Constraint::Ratio(2, 3), // CPU HEAT (8 cols)
    ])
    .spacing(1)
    .split(rows[0]);

    if row1.len() >= 2 {
        let inner = section_header(
            f,
            row1[0],
            "󰈀 STATUS",
            theme,
            focused == Some(PanelId::Network),
        );
        status::render(f, inner, theme);

        let inner = section_header(
            f,
            row1[1],
            "\u{f0ee0} CPU HEAT",
            theme,
            focused == Some(PanelId::Cpu),
        );
        heatmap::render(f, inner, theme);
    }

    // ── Row 2: SECONDARY ──
    let row2 = Layout::horizontal([
        Constraint::Ratio(1, 3),
        Constraint::Ratio(1, 3),
        Constraint::Ratio(1, 3),
    ])
    .spacing(1)
    .split(rows[1]);

    if row2.len() >= 3 {
        let inner = section_header(
            f,
            row2[0],
            "\u{f0ee0} CORES",
            theme,
            focused == Some(PanelId::Cpu),
        );
        cores::render(f, inner, theme);

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
            "󰋼 ANALYTICS",
            theme,
            focused == Some(PanelId::Cpu),
        );
        analytics::render_compact(f, inner, theme, sum);
    }

    // ── Row 3: TERTIARY ──
    let row3 = Layout::horizontal([Constraint::Ratio(1, 4), Constraint::Ratio(3, 4)])
        .spacing(1)
        .split(rows[2]);

    if row3.len() >= 2 {
        let inner = section_header(
            f,
            row3[0],
            "󰥔 CLOCK",
            theme,
            focused == Some(PanelId::Clock),
        );
        clock::render(f, inner, theme);

        let inner = section_header(
            f,
            row3[1],
            "󰝚 VISUALIZER",
            theme,
            focused == Some(PanelId::Visualizer),
        );
        music_viz::render(f, inner, theme, _tick);
    }
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
