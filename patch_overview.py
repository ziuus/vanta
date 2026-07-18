content = open("src/screens/overview.rs").read()

import re

old_header = """fn section_header(f: &mut Frame, area: Rect, label: &str, theme: &app::Theme, focused: bool) -> Rect {
    let c = if focused { theme.focus } else { theme.accent };
    let rows = Layout::vertical([Constraint::Min(1), Constraint::Min(1)]).split(area);
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            format!(" {} ", label),
            Style::default().fg(c).add_modifier(Modifier::BOLD),
        )))
        .style(Style::default().bg(theme.bg)),
        rows[0],
    );
    rows[1]
}"""

new_header = """use ratatui::widgets::{Block, Borders, BorderType};

fn section_header(f: &mut Frame, area: Rect, label: &str, theme: &app::Theme, focused: bool) -> Rect {
    let title_color = if focused { theme.focus } else { theme.accent };
    let border_color = if focused { theme.focus } else { theme.dim };
    
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color))
        .title(Span::styled(
            format!(" {} ", label),
            Style::default().fg(title_color).add_modifier(Modifier::BOLD),
        ));
        
    let inner = block.inner(area);
    f.render_widget(block, area);
    inner
}"""

content = content.replace(old_header, new_header)
open("src/screens/overview.rs", "w").write(content)
