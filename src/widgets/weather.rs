use ratatui::layout::{Alignment, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::monitors::weather;
use crate::theme::Theme;

pub fn render(f: &mut Frame, area: Rect, theme: &Theme) {
    if area.height < 5 || area.width < 15 {
        return;
    }

    let snap = weather::snapshot();

    let mut lines = Vec::new();

    if !snap.ready {
        lines.push(Line::from(vec![Span::styled(
            "Fetching weather...",
            Style::default().fg(theme.dim),
        )]));
        let out = vec![Line::default(); (area.height as usize).saturating_sub(1) / 2];
        let mut final_out = out;
        final_out.extend(lines);
        f.render_widget(Paragraph::new(final_out).alignment(Alignment::Center), area);
        return;
    }

    let (icon, desc) = match snap.condition_code {
        0 => (
            ["      ", " \\ ▂ / ", " - ☼ - ", " / ▅ \\ ", "      "],
            "Clear",
        ),
        1..=3 => (
            ["      ", "  ☁☁☁  ", " ☁   ☁ ", "  ☁☁☁  ", "      "],
            "Cloudy",
        ),
        45 | 48 => (["      ", " ▒▒▒▒▒ ", " ▒▒▒▒▒ ", " ▒▒▒▒▒ ", "      "], "Fog"),
        51..=57 | 61..=67 | 80..=82 => (
            ["      ", "  ☁☁☁  ", "  ▅ ▅ ▅", "  ▅ ▅ ▅", "      "],
            "Rain",
        ),
        71..=77 | 85..=86 => (
            ["      ", "  ☁☁☁  ", "  * * *", "  * * *", "      "],
            "Snow",
        ),
        95..=99 => (
            ["      ", "  ☁☁☁  ", "   ⚡   ", "  ⚡    ", "      "],
            "Storm",
        ),
        _ => (
            ["      ", " \\ ▂ / ", " - ☼ - ", " / ▅ \\ ", "      "],
            "Unknown",
        ),
    };

    let temp_str = format!("{:.0}°C", snap.temp_c);

    let padding_y = (area.height.saturating_sub(5)) / 2;
    for _ in 0..padding_y {
        lines.push(Line::default());
    }

    let h_pad = " ".repeat((area.width.saturating_sub(25) / 2) as usize);

    for (i, &icon_row) in icon.iter().enumerate() {
        let mut spans = vec![Span::raw(h_pad.clone())];

        spans.push(Span::styled(
            icon_row,
            Style::default().fg(if snap.is_day || i == 1 {
                theme.accent
            } else {
                theme.surface
            }),
        ));

        spans.push(Span::raw("   "));

        if i == 1 {
            spans.push(Span::styled(
                snap.location.clone(),
                Style::default().fg(theme.text),
            ));
        } else if i == 2 {
            spans.push(Span::styled(
                temp_str.clone(),
                Style::default().fg(theme.accent),
            ));
        } else if i == 3 {
            spans.push(Span::styled(
                desc.to_string(),
                Style::default().fg(theme.dim),
            ));
        }

        lines.push(Line::from(spans));
    }

    f.render_widget(Paragraph::new(lines), area);
}
