content = open("src/widgets/widget.rs").read()

import re

old_imports = """use ratatui::layout::Rect;
use ratatui::Frame;"""

new_imports = """use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::Span;
use ratatui::widgets::{Block, Borders, BorderType};
use ratatui::Frame;"""

content = content.replace(old_imports, new_imports)

helper = """// ── Trait implementations ──

fn draw_border(f: &mut Frame, area: Rect, title: &str, theme: &Theme) -> Rect {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.dim))
        .title(Span::styled(format!(" {} ", title), Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)));
    let inner = block.inner(area);
    f.render_widget(block, area);
    inner
}
"""

content = content.replace("// ── Trait implementations ──", helper)

# Now wrap every cpu::render, memory::render etc.
content = re.sub(r'cpu::render\(f, area, theme\);', r'let inner = draw_border(f, area, self.label(), theme); cpu::render(f, inner, theme);', content)
content = re.sub(r'memory::render\(f, area, theme\);', r'let inner = draw_border(f, area, self.label(), theme); memory::render(f, inner, theme);', content)
content = re.sub(r'disk::render\(f, area, theme\);', r'let inner = draw_border(f, area, self.label(), theme); disk::render(f, inner, theme);', content)
content = re.sub(r'network::render\(f, area, theme\);', r'let inner = draw_border(f, area, self.label(), theme); network::render(f, inner, theme);', content)
content = re.sub(r'gpu::render\(f, area, theme\);', r'let inner = draw_border(f, area, self.label(), theme); gpu::render(f, inner, theme);', content)
content = re.sub(r'system_info::render\(f, area, theme\);', r'let inner = draw_border(f, area, self.label(), theme); system_info::render(f, inner, theme);', content)
content = re.sub(r'clock::render\(f, area, theme\);', r'let inner = draw_border(f, area, self.label(), theme); clock::render(f, inner, theme);', content)
content = re.sub(r'calendar::render\(f, area, theme, states\.calendar_month_offset\);', r'let inner = draw_border(f, area, self.label(), theme); calendar::render(f, inner, theme, states.calendar_month_offset);', content)
content = re.sub(r'media::render\(f, area, theme\);', r'let inner = draw_border(f, area, self.label(), theme); media::render(f, inner, theme);', content)
content = re.sub(r'music_viz::render\(f, area, theme, tick\);', r'let inner = draw_border(f, area, self.label(), theme); music_viz::render(f, inner, theme, tick);', content)
content = re.sub(r'cmatrix::render\(f, area, tick\);', r'let inner = draw_border(f, area, self.label(), _theme); cmatrix::render(f, inner, tick);', content)
content = re.sub(r'settings::render\(f, area, theme, config\);', r'let inner = draw_border(f, area, self.label(), theme); settings::render(f, inner, theme, config);', content)
content = re.sub(r'processes::render\(\n\s*f,\n\s*area,', r'let inner = draw_border(f, area, self.label(), theme);\n        processes::render(\n            f,\n            inner,', content)

open("src/widgets/widget.rs", "w").write(content)
