pub mod aesthetic;
pub mod dashboard;
pub mod help;
pub mod monitor;

use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph};
use ratatui::Frame;

use crate::theme::Theme;

/// Panel chrome shared by every page: rounded border, small-caps title,
/// accent highlight when focused. Returns the inner area.
pub fn panel(f: &mut Frame, area: Rect, title: &str, theme: &Theme, focused: bool) -> Rect {
    let (border, text) = if focused {
        (theme.accent, theme.accent)
    } else {
        (theme.surface, theme.dim)
    };
    let title_style = Style::default().fg(text).add_modifier(Modifier::BOLD);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border))
        .title(Span::styled(format!(" {} ", title), title_style));
    let inner = block.inner(area);
    f.render_widget(block, area);
    inner
}

/// Centred notice for when a page can't fit. `need` is the page area; the
/// message speaks in terminal size (page + title and status bars).
pub fn too_small(f: &mut Frame, area: Rect, theme: &Theme, need: (u16, u16)) {
    let term = f.area();
    let msg = format!(
        "terminal too small — need {}×{}, have {}×{}",
        need.0,
        need.1 + 2,
        term.width,
        term.height
    );
    let y = area.y + area.height / 2;
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            msg,
            Style::default().fg(theme.dim),
        )))
        .alignment(Alignment::Center),
        Rect::new(area.x, y, area.width, 1),
    );
}
