use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;

use crate::app::Theme;
use crate::config::Config;

/// Render the Settings screen.
///
/// Shows current theme, mode, keyboard shortcuts, and config path.
/// Theme changes are handled by App::toggle_theme / App::set_theme,
/// and persisted to disk automatically.
pub fn render(f: &mut Frame, area: Rect, theme: &Theme, config: &Config) {
    let bg = theme.bg;
    let text = theme.text;
    let dim = theme.dim;
    let accent = theme.accent;
    let config_theme_name = &config.ui.theme;

    let lines = vec![
        Line::from(Span::styled(" ⚙  Settings", Style::default().fg(accent).bg(bg))),
        Line::from(""),
        Line::from(vec![
            Span::styled(" Current theme:  ", Style::default().fg(dim).bg(bg)),
            Span::styled(config_theme_name, Style::default().fg(text).bg(bg)),
        ]),
        Line::from(vec![
            Span::styled(" Available:      ", Style::default().fg(dim).bg(bg)),
            Span::styled("dark, light, dracula, solarized-light", Style::default().fg(text).bg(bg)),
        ]),
        Line::from(vec![
            Span::styled(" Toggle:         ", Style::default().fg(dim).bg(bg)),
            Span::styled("[T] to cycle through themes", Style::default().fg(text).bg(bg)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(" Dashboard mode: ", Style::default().fg(dim).bg(bg)),
            Span::styled("press [1]–[6] to switch", Style::default().fg(text).bg(bg)),
        ]),
        Line::from(vec![
            Span::styled("  1  ", Style::default().fg(accent).bg(bg)),
            Span::styled("Overview     ", Style::default().fg(dim).bg(bg)),
            Span::styled("2  ", Style::default().fg(accent).bg(bg)),
            Span::styled("Monitor", Style::default().fg(dim).bg(bg)),
        ]),
        Line::from(vec![
            Span::styled("  3  ", Style::default().fg(accent).bg(bg)),
            Span::styled("Processes    ", Style::default().fg(dim).bg(bg)),
            Span::styled("4  ", Style::default().fg(accent).bg(bg)),
            Span::styled("Media", Style::default().fg(dim).bg(bg)),
        ]),
        Line::from(vec![
            Span::styled("  5  ", Style::default().fg(accent).bg(bg)),
            Span::styled("Aesthetic    ", Style::default().fg(dim).bg(bg)),
            Span::styled("6  ", Style::default().fg(accent).bg(bg)),
            Span::styled("Settings", Style::default().fg(dim).bg(bg)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(" Theme config : ui.theme in ", Style::default().fg(dim).bg(bg)),
            Span::styled("~/.config/vanta/config.toml", Style::default().fg(text).bg(bg)),
        ]),
        Line::from(Span::styled(" Theme changes are saved immediately.", Style::default().fg(dim).bg(bg))),
        Line::from(""),
        Line::from(Span::styled(" [q] Quit    [T] Cycle theme", Style::default().fg(dim).bg(bg))),
    ];

    let widget = Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL).style(Style::default().fg(accent).bg(bg)))
        .style(Style::default().bg(bg))
        .wrap(Wrap { trim: false });

    f.render_widget(widget, area);
}