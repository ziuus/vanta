content = open("src/monitors/system_info.rs").read()

import re

old_render = """    // Single compact line: OS · hostname · CPU · GPU · shell
    let parts: Vec<String> = [Some(os_short), Some(&hostname), Some(&cpu_short), Some(&gpu), Some(&shell)]
        .into_iter()
        .flatten()
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect();
    let line = parts.join("  ·  ");

    f.render_widget(
        Paragraph::new(Line::from(Span::styled(line, Style::default().fg(theme.text))))
            .style(Style::default().bg(theme.bg)),
        area,
    );"""

new_render = """    let mut lines = Vec::new();
    
    let key_style = Style::default().fg(theme.accent);
    let val_style = Style::default().fg(theme.text);

    lines.push(Line::from(vec![
        Span::styled(format!("{:>8} ", "OS"), key_style),
        Span::styled(os_short, val_style),
    ]));
    lines.push(Line::from(vec![
        Span::styled(format!("{:>8} ", "Host"), key_style),
        Span::styled(hostname, val_style),
    ]));
    lines.push(Line::from(vec![
        Span::styled(format!("{:>8} ", "CPU"), key_style),
        Span::styled(cpu_short, val_style),
    ]));
    if !gpu.is_empty() {
        lines.push(Line::from(vec![
            Span::styled(format!("{:>8} ", "GPU"), key_style),
            Span::styled(gpu, val_style),
        ]));
    }
    lines.push(Line::from(vec![
        Span::styled(format!("{:>8} ", "Shell"), key_style),
        Span::styled(shell, val_style),
    ]));

    f.render_widget(
        Paragraph::new(lines).style(Style::default().bg(theme.bg)),
        area,
    );"""

content = content.replace(old_render, new_render)
open("src/monitors/system_info.rs", "w").write(content)
