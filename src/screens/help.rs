use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Paragraph};
use ratatui::Frame;

use crate::config::{self, Config};
use crate::theme::{Theme, THEME_NAMES};

fn centered(area: Rect, w: u16, h: u16) -> Rect {
    let w = w.min(area.width);
    let h = h.min(area.height);
    Rect::new(
        area.x + (area.width - w) / 2,
        area.y + (area.height - h) / 2,
        w,
        h,
    )
}

/// Floating keybind reference. Any key closes it.
pub fn render(f: &mut Frame, area: Rect, theme: &Theme, config: &Config) {
    let bg = theme.surface;
    let base = Style::default().bg(bg);
    let key = |k: &str, d: &str| {
        Line::from(vec![
            Span::styled(
                format!("   {:<9}", k),
                base.fg(theme.accent).add_modifier(Modifier::BOLD),
            ),
            Span::styled(d.to_string(), base.fg(theme.text)),
        ])
    };
    let head = |s: &str| {
        Line::from(Span::styled(
            format!(" {}", s),
            base.fg(theme.secondary).add_modifier(Modifier::BOLD),
        ))
    };
    let blank = Line::from(Span::styled("", base));

    let themes = THEME_NAMES.join(", ");
    let lines = vec![
        head("pages"),
        key("1 2 3", "dashboard · monitor · aesthetic"),
        blank.clone(),
        head("global"),
        key(
            "tab ⇧tab",
            "cycle panel focus        esc  clear focus / unzoom",
        ),
        key("enter", "zoom focused panel to full page"),
        key("T", "next theme               v    visualizer style"),
        key("+ -", "sample faster / slower   ?    this help"),
        key("q", "quit"),
        blank.clone(),
        head("media (any page)"),
        key("space", "play / pause             n p  next / previous"),
        key("< >", "volume down / up"),
        blank.clone(),
        head("processes (monitor page)"),
        key(
            "↑ ↓ pgup",
            "select                   /    filter (enter keeps, esc clears)",
        ),
        key("s r", "sort field / reverse     t    tree view   ← →  fold"),
        key(
            "c",
            "toggle full command      k K  SIGTERM / SIGKILL (press twice)",
        ),
        key("x", "cancel a pending kill"),
        blank.clone(),
        head("calendar (when focused)"),
        key(
            "← →",
            "month                    ↑ ↓  year        home today",
        ),
        blank.clone(),
        Line::from(vec![
            Span::styled("   theme    ", base.fg(theme.dim)),
            Span::styled(config.ui.theme.clone(), base.fg(theme.text)),
            Span::styled(format!("   ({})", themes), base.fg(theme.dim)),
        ]),
        Line::from(vec![
            Span::styled("   config   ", base.fg(theme.dim)),
            Span::styled(config::config_path(), base.fg(theme.text)),
        ]),
    ];

    let w = lines
        .iter()
        .map(|l| {
            l.spans
                .iter()
                .map(|s| s.content.chars().count())
                .sum::<usize>()
        })
        .max()
        .unwrap_or(60) as u16
        + 4;
    let box_area = centered(area, w.min(area.width), lines.len() as u16 + 2);
    f.render_widget(Clear, box_area);
    f.render_widget(
        Paragraph::new(lines).style(base).block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(theme.accent))
                .title(Span::styled(
                    " vanta · help ",
                    Style::default()
                        .fg(theme.accent)
                        .add_modifier(Modifier::BOLD),
                ))
                .style(base),
        ),
        box_area,
    );
}
