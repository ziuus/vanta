use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph, Wrap};
use ratatui::Frame;

use crate::app::Theme;
use crate::config::Config;

/// Draw a titled rounded border and return the inner area.
/// Shared by full-screen mode views.
pub fn block_inner(f: &mut Frame, area: Rect, label: &str, theme: &Theme, focused: bool) -> Rect {
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
                .fg(if focused { theme.accent } else { theme.dim })
                .add_modifier(Modifier::BOLD),
        ));
    let inner = block.inner(area);
    f.render_widget(block, area);
    inner
}

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
    let green = theme.green;
    let config_theme_name = &config.ui.theme;

    let mode_line = |key: char, name: &str| {
        Line::from(vec![
            Span::styled(format!("   {}  ", key), Style::default().fg(accent).bg(bg)),
            Span::styled(name.to_string(), Style::default().fg(dim).bg(bg)),
        ])
    };

    let lines = vec![
        Line::from(Span::styled(
            " ⚙  Settings",
            Style::default().fg(accent).bg(bg),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled(" Current theme:  ", Style::default().fg(dim).bg(bg)),
            Span::styled(config_theme_name, Style::default().fg(green).bg(bg)),
        ]),
        Line::from(vec![
            Span::styled(" Available:      ", Style::default().fg(dim).bg(bg)),
            Span::styled(
                "dark, light, dracula, solarized-light",
                Style::default().fg(text).bg(bg),
            ),
        ]),
        Line::from(vec![
            Span::styled(" Toggle:         ", Style::default().fg(dim).bg(bg)),
            Span::styled(
                "[T] to cycle through themes",
                Style::default().fg(text).bg(bg),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(" Dashboard modes ", Style::default().fg(dim).bg(bg)),
            Span::styled("press [1]–[6] to switch", Style::default().fg(text).bg(bg)),
        ]),
        mode_line('1', "Overview     full monitoring grid"),
        mode_line('2', "Monitor      focused hardware health"),
        mode_line('3', "Processes    full-width process table"),
        mode_line('4', "Media        visualizer + player controls"),
        mode_line('5', "Aesthetic    clock, calendar, matrix"),
        mode_line('6', "Settings     this screen"),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                " Theme config : ui.theme in ",
                Style::default().fg(dim).bg(bg),
            ),
            Span::styled(
                "~/.config/vanta/config.toml",
                Style::default().fg(text).bg(bg),
            ),
        ]),
        Line::from(Span::styled(
            " Theme changes are saved immediately.",
            Style::default().fg(dim).bg(bg),
        )),
        Line::from(""),
        Line::from(Span::styled(
            " [q] Quit    [T] Cycle theme    [Tab] Focus",
            Style::default().fg(dim).bg(bg),
        )),
    ];

    let widget = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(accent))
                .style(Style::default().fg(accent).bg(bg)),
        )
        .style(Style::default().bg(bg))
        .wrap(Wrap { trim: false });

    f.render_widget(widget, area);
}
