use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Paragraph};
use ratatui::Frame;

use crate::config::{self, Config};
use crate::theme::Theme;

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

/// (heading, [(keys, description)])
type Section = (&'static str, &'static [(&'static str, &'static str)]);

const SECTIONS: &[Section] = &[
    (
        "pages",
        &[
            ("1 2 3 4", "overview · monitor · ambient · focus"),
            ("tab ⇧tab", "cycle panel focus"),
            ("enter", "zoom focused panel"),
            ("esc", "clear focus / unzoom"),
        ],
    ),
    (
        "look",
        &[
            ("T", "next theme"),
            ("v", "visualizer style"),
            ("g m G", "gauge · meter · graph style"),
            ("S ,", "settings"),
            ("+ -", "sample faster / slower"),
            ("? q", "this help · quit"),
        ],
    ),
    (
        "media (any page)",
        &[
            ("space", "play / pause"),
            ("n p", "next / previous track"),
            ("< >", "volume down / up"),
        ],
    ),
    (
        "ambient",
        &[
            ("← →", "previous / next scene"),
            ("r", "pause / resume rotation"),
            ("i", "set pinned image (gallery)"),
            ("o O", "pause motion · 3d mode"),
            ("[ ]", "motion slower / faster"),
        ],
    ),
    (
        "focus timer (focused)",
        &[("space", "start / pause"), ("r s", "reset · skip phase")],
    ),
    (
        "calendar (focused)",
        &[("← →", "month"), ("↑ ↓", "year"), ("home", "today")],
    ),
    (
        "processes (monitor)",
        &[
            ("↑ ↓ pgup", "select"),
            ("/", "filter (enter keeps, esc clears)"),
            ("s r", "sort field · reverse"),
            ("t ← →", "tree view · fold"),
            ("c", "toggle full command"),
            ("k K", "SIGTERM · SIGKILL (twice)"),
            ("x", "cancel a pending kill"),
        ],
    ),
];

/// Floating keybind reference. Any key closes it. Sections flow into two
/// columns when a single column would not fit the terminal height.
pub fn render(f: &mut Frame, area: Rect, theme: &Theme, config: &Config) {
    let bg = theme.surface;
    let base = Style::default().bg(bg);
    let blank = || Line::from(Span::styled("", base));

    let section_lines = |(head, keys): &Section| -> Vec<Line<'static>> {
        let mut v = vec![Line::from(Span::styled(
            format!(" {}", head),
            base.fg(theme.secondary),
        ))];
        v.extend(keys.iter().map(|(k, d)| {
            Line::from(vec![
                Span::styled(format!("   {:<10}", k), base.fg(theme.accent)),
                Span::styled(d.to_string(), base.fg(theme.text)),
            ])
        }));
        v
    };
    let blocks: Vec<Vec<Line>> = SECTIONS.iter().map(section_lines).collect();
    let footer = vec![
        Line::from(vec![
            Span::styled("   theme    ", base.fg(theme.dim)),
            Span::styled(config.ui.theme.clone(), base.fg(theme.text)),
            Span::styled(
                format!(
                    "   ({} themes · T cycles)",
                    crate::theme::theme_names().len()
                ),
                base.fg(theme.dim),
            ),
        ]),
        Line::from(vec![
            Span::styled("   config   ", base.fg(theme.dim)),
            Span::styled(config::config_path(), base.fg(theme.text)),
        ]),
    ];

    let line_w = |l: &Line| {
        l.spans
            .iter()
            .map(|s| s.content.chars().count())
            .sum::<usize>()
    };
    let col_w = blocks.iter().flatten().map(line_w).max().unwrap_or(40) + 2;
    let single: usize = blocks.iter().map(|b| b.len() + 1).sum::<usize>() + footer.len();

    // Two columns if one doesn't fit and there's room side by side.
    let two_col = single + 2 > area.height as usize && (col_w * 2 + 4) <= area.width as usize;
    let mut lines: Vec<Line> = Vec::new();
    if two_col {
        let half = single / 2;
        let (mut left, mut right): (Vec<Line>, Vec<Line>) = (Vec::new(), Vec::new());
        for b in blocks {
            let target = if left.len() < half {
                &mut left
            } else {
                &mut right
            };
            target.extend(b);
            target.push(blank());
        }
        let rows = left.len().max(right.len());
        for i in 0..rows {
            let mut spans: Vec<Span> = Vec::new();
            if let Some(l) = left.get(i) {
                spans.extend(l.spans.clone());
                spans.push(Span::styled(
                    " ".repeat(col_w.saturating_sub(line_w(l))),
                    base,
                ));
            } else {
                spans.push(Span::styled(" ".repeat(col_w), base));
            }
            if let Some(r) = right.get(i) {
                spans.extend(r.spans.clone());
            }
            lines.push(Line::from(spans));
        }
    } else {
        for b in blocks {
            lines.extend(b);
            lines.push(blank());
        }
    }
    lines.extend(footer);

    let w = lines.iter().map(line_w).max().unwrap_or(60) as u16 + 4;
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
                    Style::default().fg(theme.accent),
                ))
                .style(base),
        ),
        box_area,
    );
}
