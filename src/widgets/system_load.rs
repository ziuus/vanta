use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{LineGauge, Paragraph};
use ratatui::Frame;

use crate::app::Theme;

/// Render minimal system load bars
pub fn render(f: &mut Frame, area: Rect, theme: &Theme, metrics: &[(&str, f64, String, Color)]) {
    if area.height < metrics.len() as u16 * 2 - 1 {
        return;
    }

    let mut y = area.y;
    for (label, pct, val_str, col) in metrics {
        if y >= area.y + area.height {
            break;
        }

        // Header line: Label on left, Value on right
        let pad = (area.width as usize).saturating_sub(label.len() + val_str.len());
        f.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(format!("{}", label), Style::default().fg(theme.text)),
                Span::raw(" ".repeat(pad)),
                Span::styled(val_str.clone(), Style::default().fg(*col)),
            ])),
            Rect::new(area.x, y, area.width, 1),
        );
        y += 1;

        if y >= area.y + area.height {
            break;
        }

        // LineGauge
        f.render_widget(
            LineGauge::default()
                .filled_style(Style::default().fg(*col))
                .unfilled_style(Style::default().fg(theme.dim))
                .line_set(ratatui::symbols::line::THICK)
                .ratio((pct / 100.0).clamp(0.0, 1.0)),
            Rect::new(area.x, y, area.width, 1),
        );
        y += 1; // bar is 1 line, plus we could add spacing, but let's just make it dense
    }
}
