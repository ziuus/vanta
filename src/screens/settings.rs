use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Paragraph, Wrap};
use ratatui::Frame;

use crate::app::Theme;
use crate::config::Config;

/// Compute a centered rect `w`×`h` within `area` (clamped to fit).
fn centered(area: Rect, w: u16, h: u16) -> Rect {
    let w = w.min(area.width);
    let h = h.min(area.height);
    let x = area.x + (area.width.saturating_sub(w)) / 2;
    let y = area.y + (area.height.saturating_sub(h)) / 2;
    Rect::new(x, y, w, h)
}

/// Render the help/settings overlay as a centered floating box.
///
/// Toggled with `?`; floats over whatever page is active. Theme changes are
/// handled by App::toggle_theme / App::set_theme and persisted to disk.
pub fn render_overlay(f: &mut Frame, area: Rect, theme: &Theme, config: &Config) {
    let bg = theme.surface;
    let text = theme.text;
    let dim = theme.dim;
    let accent = theme.accent;
    let green = theme.green;
    let config_theme_name = &config.ui.theme;

    let kv = |label: &str, value: &str| {
        Line::from(vec![
            Span::styled(format!("  {:<10}", label), Style::default().fg(dim).bg(bg)),
            Span::styled(value.to_string(), Style::default().fg(text).bg(bg)),
        ])
    };
    let key = |k: &str, desc: &str| {
        Line::from(vec![
            Span::styled(format!("   {:<8}", k), Style::default().fg(accent).bg(bg)),
            Span::styled(desc.to_string(), Style::default().fg(text).bg(bg)),
        ])
    };
    let head = |s: &str| {
        Line::from(Span::styled(
            format!(" {s}"),
            Style::default().fg(green).bg(bg).add_modifier(Modifier::BOLD),
        ))
    };

    let lines = vec![
        Line::from(Span::styled(
            " ⚙  vanta — help & settings",
            Style::default().fg(accent).bg(bg).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        head("Pages"),
        key("1", "Dashboard   monitoring overview + clock/calendar/media/viz"),
        key("2", "Monitor     btop-style detail + full process table"),
        key("3", "Aesthetic   clock, calendar, visualizer, matrix, 3D demo"),
        Line::from(""),
        head("Global"),
        key("T", "cycle theme (saved to config)"),
        key("v", "cycle visualizer style (bars / mirror / wave)"),
        key("Tab", "cycle panel focus     Esc  clear focus / close this"),
        key("?", "toggle this overlay   q    quit"),
        Line::from(""),
        head("Processes (Monitor page)"),
        key("s", "sort   / search   t tree   i info   k kill (SIGTERM)"),
        Line::from(""),
        head("Theme"),
        kv("current", config_theme_name),
        kv("available", "dark, light, dracula, solarized-light"),
        kv("config", "~/.config/vanta/config.toml"),
    ];

    let box_area = centered(area, 64, lines.len() as u16 + 2);

    // Clear whatever's underneath so the box is opaque.
    f.render_widget(Clear, box_area);

    let widget = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(accent))
                .title(Span::styled(
                    " ? ",
                    Style::default().fg(accent).add_modifier(Modifier::BOLD),
                ))
                .style(Style::default().bg(bg)),
        )
        .style(Style::default().bg(bg))
        .wrap(Wrap { trim: false });

    f.render_widget(widget, box_area);
}
