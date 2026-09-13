import re
with open("src/monitors/network.rs", "r") as f:
    content = f.read()

helper = """        if split[1].height > 0 {
            let u64_hist: Vec<u64> = hist.iter().map(|&v| v as u64).collect();
            f.render_widget(
                Sparkline::default()
                    .data(&u64_hist)
                    .max(peak as u64)
                    .style(Style::default().fg(color)),
                split[1],
            );
        }"""
content = re.sub(r'        if split\[1\]\.height > 0 \{\n            f\.render_widget\(\n                BlockGraph::new\(hist\)\.max\(peak\)\.colors\(color, color, color\),\n                split\[1\],\n            \);\n        \}', helper, content, count=1)

with open("src/monitors/network.rs", "w") as f:
    f.write(content)
