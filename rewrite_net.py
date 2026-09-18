import sys

content = open('src/monitors/network.rs').read()

start = content.find("pub fn render(f: &mut Frame, area: Rect, theme: &Theme) {")
end = content.find("}\n", content.rfind("}")) + 2 # Find end of render function

new_render = """pub fn render(f: &mut Frame, area: Rect, theme: &Theme) {
    if area.height < 3 {
        return;
    }
    let (rx_h, tx_h, snap) = {
        let n = NET.lock().unwrap();
        (
            n.rx.recent(area.width as usize),
            n.tx.recent(area.width as usize),
            NetSnapshot {
                rx_kbps: n.rx_kbps,
                tx_kbps: n.tx_kbps,
                rx_total: n.rx_total,
                tx_total: n.tx_total,
            },
        )
    };

    let split = Layout::vertical([Constraint::Length(1), Constraint::Length(1), Constraint::Min(0)]).split(area);
    let peak = rx_h.iter().chain(tx_h.iter()).copied().fold(0.0, f64::max).max(1.0);

    let mk_line = |arrow: &str, rate: f64, peak: f64, color, total| {
        let mut head = Line::from(vec![
            Span::styled(format!("{} ", arrow), Style::default().fg(color)),
            Span::styled(
                format!("{:>10}", meter::fmt_kbps(rate)),
                Style::default().fg(color),
            ),
            Span::styled(
                format!("  peak {}", meter::fmt_kbps(peak)),
                Style::default().fg(theme.dim),
            ),
        ]);
        if area.width >= 44 {
            head.spans.push(Span::styled(
                format!("  total {}", meter::fmt_bytes(total)),
                Style::default().fg(theme.dim),
            ));
        }
        head
    };

    f.render_widget(Paragraph::new(mk_line("↓", snap.rx_kbps, peak, theme.accent, snap.rx_total)), split[0]);
    f.render_widget(Paragraph::new(mk_line("↑", snap.tx_kbps, peak, theme.secondary, snap.tx_total)), split[1]);

    if split[2].height > 0 {
        let rx_cur = rx_h.last().copied().unwrap_or(0.0);
        let rx_dyn = if peak > 0.0 { theme.usage(rx_cur / peak * 100.0) } else { theme.accent };
        let tx_cur = tx_h.last().copied().unwrap_or(0.0);
        let tx_dyn = if peak > 0.0 { theme.usage(tx_cur / peak * 100.0) } else { theme.secondary };
        
        f.render_widget(
            BlockGraph::new(&rx_h)
                .data2(&tx_h)
                .max(peak)
                .colors(rx_dyn, theme.yellow, theme.red)
                .colors2(tx_dyn, theme.yellow, theme.red),
            split[2],
        );
    }
}"""

# Actually just find the function bounds accurately
# It's at the end of the file.
lines = content.split('\n')
start_idx = 0
for i, line in enumerate(lines):
    if line.startswith("pub fn render(f: &mut Frame, area: Rect, theme: &Theme) {"):
        start_idx = i
        break

new_content = '\n'.join(lines[:start_idx]) + '\n' + new_render + '\n'
open('src/monitors/network.rs', 'w').write(new_content)
print("Rewrote network.rs render function")
