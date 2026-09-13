import re
with open("src/screens/mod.rs", "r") as f:
    content = f.read()

panel_full = """pub fn panel_full(
    f: &mut Frame,
    area: Rect,
    title: &str,
    right_title: Option<&str>,
    footer: Option<&str>,
    theme: &Theme,
    focused: bool,
) -> Rect {
    let (border, text) = if focused {
        (theme.accent, theme.accent)
    } else {
        (theme.surface, theme.dim)
    };
    
    let title_style = Style::default().fg(text).add_modifier(Modifier::BOLD);
    let mut block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border))
        .title_top(Line::from(Span::styled(format!(" {} ", title), title_style)));

    if let Some(rt) = right_title {
        let rt_style = Style::default().fg(theme.dim);
        block = block.title_top(Line::from(Span::styled(format!(" {} ", rt), rt_style)).alignment(Alignment::Right));
    }
    
    if let Some(ft) = footer {
        if focused {
            let ft_style = Style::default().fg(theme.accent);
            block = block.title_bottom(Line::from(Span::styled(format!(" {} ", ft), ft_style)).alignment(Alignment::Center));
        }
    }
    
    let inner = block.inner(area);
    f.render_widget(block, area);
    inner
}

pub fn panel(f: &mut Frame, area: Rect, title: &str, theme: &Theme, focused: bool) -> Rect {
    panel_full(f, area, title, None, None, theme, focused)
}"""

# Need to replace the whole block again. Since we already replaced it, let's search for pub fn panel_full and replace it.
content = re.sub(r'pub fn panel_full.*?None, None, theme, focused\)\n\}', panel_full, content, flags=re.DOTALL)
# Also remove `use ratatui::widgets::block::{Position, Title};`
content = re.sub(r'use ratatui::widgets::block::\{Position, Title\};\n\n', '', content)

with open("src/screens/mod.rs", "w") as f:
    f.write(content)
