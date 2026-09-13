import re
with open("src/monitors/cpu.rs", "r") as f:
    content = f.read()

content = re.sub(r'use crate::widgets::block_graph::BlockGraph;', 'use ratatui::widgets::Sparkline;', content, count=1)

sparkline_str = """    let hist_u64: Vec<u64> = hist.iter().map(|&v| v as u64).collect();
    f.render_widget(
        Sparkline::default()
            .data(&hist_u64)
            .max(100)
            .style(Style::default().fg(theme.accent)),
        chunks[1],
    );"""
content = re.sub(r'    f\.render_widget\(\n        BlockGraph::new\(&hist\)\n            \.max\(100\.0\)\n            \.colors\(theme\.accent, theme\.yellow, theme\.red\),\n        chunks\[1\],\n    \);', sparkline_str, content, count=1)

with open("src/monitors/cpu.rs", "w") as f:
    f.write(content)
