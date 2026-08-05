use ratatui::layout::{Alignment, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::app::Theme;

const TIPS: &[&str] = &[
    "Press `v` to cycle visualizer styles",
    "Press `T` to change themes",
    "Press `?` for help and keybinds",
    "Press `1`, `2`, `3` to switch pages",
    "Press Tab to cycle panel focus",
    "Configure widgets in ~/.config/vanta/config.toml",
];

const COW: [&str; 6] = [
    r"  \   ^__^",
    r"   \  (oo)\_______",
    r"      (__)\       )\/\",
    r"          ||----w |",
    r"          ||     ||",
    "",
];

pub fn render(f: &mut Frame, area: Rect, theme: &Theme) {
    if area.height < 3 || area.width < 24 {
        return;
    }
    let with_cow = area.height >= 9;

    // Pick a tip based on the current second (cycles through all tips every minute)
    let tip_idx = (std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
        / 10) as usize
        % TIPS.len();
    let tip = TIPS[tip_idx];

    // Speech bubble: tips are authored short, so we truncate rather than wrap.
    // ponytail: no textwrap dep for one line of text.
    let max_tip_w = (area.width as usize).saturating_sub(6).min(40);
    let tip: String = if tip.len() > max_tip_w {
        format!("{}…", &tip[..max_tip_w.saturating_sub(1)])
    } else {
        tip.to_string()
    };
    let bubble_w = tip.len() + 4;

    let mut lines: Vec<Line<'static>> = Vec::new();

    // Top border
    lines.push(Line::from(Span::styled(
        format!(" {}", "_".repeat(bubble_w.saturating_sub(2))),
        Style::default().fg(theme.dim),
    )));

    // Tip line
    lines.push(Line::from(vec![
        Span::styled("< ", Style::default().fg(theme.dim)),
        Span::styled(tip, Style::default().fg(theme.text)),
        Span::styled(" >", Style::default().fg(theme.dim)),
    ]));

    // Bottom border
    lines.push(Line::from(Span::styled(
        format!(" {}", "-".repeat(bubble_w.saturating_sub(2))),
        Style::default().fg(theme.dim),
    )));

    // Cow (only when the panel is tall enough to hold it)
    for cow_line in COW.iter().filter(|_| with_cow) {
        lines.push(Line::from(Span::styled(
            cow_line.to_string(),
            Style::default().fg(theme.dim),
        )));
    }

    // Vertical centering
    let content_h = lines.len() as u16;
    let top = (area.height.saturating_sub(content_h)) / 2;
    let inner = Rect::new(area.x, area.y + top, area.width, content_h);

    f.render_widget(
        Paragraph::new(lines).alignment(Alignment::Center),
        inner,
    );
}
