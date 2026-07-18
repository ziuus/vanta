use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};

pub fn draw_premium_bar(
    label: &str,
    label_w: usize,
    stats: &str,
    stats_w: usize,
    pct: f64, // 0.0 to 1.0
    color: Color,
    theme_bg: Color, 
    total_width: u16,
) -> Line<'static> {
    let pct_w = 5; 
    let bracket_space = if stats.is_empty() { 3 } else { 4 }; 
    
    let fixed_w = label_w + stats_w + pct_w + bracket_space;
    let bar_w = (total_width as usize).saturating_sub(fixed_w);
    
    let filled_w = ((bar_w as f64) * pct).round() as usize;
    let empty_w = bar_w.saturating_sub(filled_w);
    
    // btop style blocks
    let bar_filled = "■".repeat(filled_w);
    let bar_empty = " ".repeat(empty_w); 
    
    let mut spans = Vec::new();
    
    if label_w > 0 {
        spans.push(Span::styled(
            format!("{:<w$}", label, w = label_w),
            Style::default().fg(Color::White), 
        ));
    }
    
    if stats_w > 0 {
        spans.push(Span::styled(
            format!(" {:>w$}", stats, w = stats_w),
            Style::default().fg(Color::Gray),
        ));
    }

    spans.push(Span::styled(" [", Style::default().fg(theme_bg)));
    spans.push(Span::styled(bar_filled, Style::default().fg(color)));
    spans.push(Span::styled(bar_empty, Style::default().fg(theme_bg)));
    spans.push(Span::styled("]", Style::default().fg(theme_bg)));
    
    spans.push(Span::styled(
        format!("{:>w$.0}%", pct * 100.0, w = pct_w - 1),
        Style::default().fg(color),
    ));
    
    Line::from(spans)
}
