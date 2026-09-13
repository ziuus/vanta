import re
with open("src/monitors/network.rs", "r") as f:
    content = f.read()

content = re.sub(r'use crate::widgets::block_graph::BlockGraph;', 'use ratatui::widgets::Sparkline;', content, count=1)

rx_sparkline = """    let rx_u64: Vec<u64> = rx_hist.iter().map(|&v| v as u64).collect();
    f.render_widget(
        Sparkline::default()
            .data(&rx_u64)
            .max(max_val as u64)
            .style(Style::default().fg(theme.green)),
        rows[1],
    );"""
content = re.sub(r'    f\.render_widget\(\n        BlockGraph::new\(&rx_hist\)\n            \.max\(max_val\)\n            \.colors\(theme\.green, theme\.green, theme\.green\),\n        rows\[1\],\n    \);', rx_sparkline, content, count=1)

tx_sparkline = """    let tx_u64: Vec<u64> = tx_hist.iter().map(|&v| v as u64).collect();
    f.render_widget(
        Sparkline::default()
            .data(&tx_u64)
            .max(max_val as u64)
            .style(Style::default().fg(theme.magenta)),
        rows[3],
    );"""
content = re.sub(r'    f\.render_widget\(\n        BlockGraph::new\(&tx_hist\)\n            \.max\(max_val\)\n            \.colors\(theme\.magenta, theme\.magenta, theme\.magenta\),\n        rows\[3\],\n    \);', tx_sparkline, content, count=1)

with open("src/monitors/network.rs", "w") as f:
    f.write(content)
