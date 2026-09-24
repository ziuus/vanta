use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::monitors::weather::{self, WeatherSnapshot};
use crate::theme::Theme;

const SPARK: [char; 8] = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

/// Single-cell condition glyph.
fn glyph(code: u8, is_day: bool) -> &'static str {
    match code {
        0 | 1 if is_day => "☀",
        0 | 1 => "☾",
        2 | 3 => "☁",
        45 | 48 => "≋",
        51..=67 | 80..=82 => "☂",
        71..=77 | 85..=86 => "❄",
        95..=99 => "ϟ",
        _ => "·",
    }
}

/// Five-row ASCII art for roomy panels.
fn art(code: u8, is_day: bool) -> [&'static str; 5] {
    match code {
        0 | 1 if is_day => [
            "  \\ │ /  ",
            "   .─.   ",
            "── ( ) ──",
            "   `─'   ",
            "  / │ \\  ",
        ],
        0 | 1 => [
            "    .─.  ",
            "   (  ·  ",
            "   (     ",
            "    `─'  ",
            "  ·    · ",
        ],
        2 => [
            "  \\ │    ",
            "  .─. ─. ",
            "─( .─(  )",
            " (___(__)",
            "         ",
        ],
        3 => [
            "         ",
            "   .──.  ",
            " .─(    ).",
            "(___.__)_)",
            "         ",
        ],
        45 | 48 => [
            "         ",
            " ~ ~ ~ ~ ",
            "  ~ ~ ~ ~",
            " ~ ~ ~ ~ ",
            "         ",
        ],
        51..=67 | 80..=82 => [
            "   .──.  ",
            " .─(    ).",
            "(___.__)_)",
            "  ʻ ʻ ʻ ʻ ",
            " ʻ ʻ ʻ ʻ  ",
        ],
        71..=77 | 85..=86 => [
            "   .──.  ",
            " .─(    ).",
            "(___.__)_)",
            "  * * * * ",
            " * * * *  ",
        ],
        95..=99 => [
            "   .──.  ",
            " .─(    ).",
            "(___.__)_)",
            "   ϟ  ϟ   ",
            "  ϟ  ϟ    ",
        ],
        _ => [
            "         ",
            "    ?    ",
            "         ",
            "         ",
            "         ",
        ],
    }
}

fn temp_color(theme: &Theme, c: f32) -> Color {
    if c >= 33.0 {
        theme.red
    } else if c >= 26.0 {
        theme.yellow
    } else if c >= 12.0 {
        theme.accent
    } else {
        theme.secondary
    }
}

fn detail_lines(s: &WeatherSnapshot, theme: &Theme, width: usize) -> Vec<Line<'static>> {
    let dim = Style::default().fg(theme.dim);
    let text = Style::default().fg(theme.text);
    let head = Line::from(vec![
        Span::styled(
            format!("{:.0}°", s.temp_c),
            Style::default()
                .fg(temp_color(theme, s.temp_c))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(format!("  {}", weather::describe(s.condition_code)), text),
    ]);
    let mut feel = vec![
        Span::styled("feels ", dim),
        Span::styled(format!("{:.0}°", s.feels_c), text),
    ];
    if width >= 24 {
        feel.push(Span::styled("  hum ", dim));
        feel.push(Span::styled(format!("{}%", s.humidity), text));
    }
    if width >= 36 {
        feel.push(Span::styled("  wind ", dim));
        feel.push(Span::styled(format!("{:.0}km/h", s.wind_kmh), text));
    }
    let mut lines = vec![head, Line::from(feel)];
    let mut range = Vec::new();
    if let (Some(hi), Some(lo)) = (s.max_c, s.min_c) {
        range.push(Span::styled("↑", dim));
        range.push(Span::styled(
            format!("{:.0}° ", hi),
            Style::default().fg(temp_color(theme, hi)),
        ));
        range.push(Span::styled("↓", dim));
        range.push(Span::styled(
            format!("{:.0}°", lo),
            Style::default().fg(temp_color(theme, lo)),
        ));
    }
    if !s.sunrise.is_empty() && width >= 28 {
        range.push(Span::styled(format!("  ☀ {}", s.sunrise), dim));
        range.push(Span::styled(format!("  ☾ {}", s.sunset), dim));
    }
    if !range.is_empty() {
        lines.push(Line::from(range));
    }
    lines
}

/// Hourly temperature sparkline plus an hour ruler underneath.
fn forecast_lines(s: &WeatherSnapshot, theme: &Theme, width: usize) -> Vec<Line<'static>> {
    // Two cells per hour reads better than one when there's room.
    let step = if width >= 48 { 2 } else { 1 };
    let hours = (width / step).min(s.hourly_c.len());
    if hours < 4 {
        return Vec::new();
    }
    let temps = &s.hourly_c[..hours];
    let (lo, hi) = temps
        .iter()
        .fold((f32::MAX, f32::MIN), |(a, b), &t| (a.min(t), b.max(t)));
    let span = (hi - lo).max(1.0);
    let mut spark = Vec::with_capacity(hours);
    for (i, &t) in temps.iter().enumerate() {
        let idx = (((t - lo) / span) * 7.0).round() as usize;
        let rainy = s.hourly_rain.get(i).is_some_and(|&p| p >= 50);
        let color = if rainy {
            theme.secondary
        } else {
            temp_color(theme, t)
        };
        spark.push(Span::styled(
            SPARK[idx.min(7)].to_string().repeat(step),
            Style::default().fg(color),
        ));
    }
    let mut ruler = String::new();
    let mut col = 0;
    while col < hours {
        let hour = (s.hour0 as usize + col) % 24;
        let label = if col == 0 {
            "now".to_string()
        } else {
            format!("{:02}", hour)
        };
        let cell = col * step;
        if ruler.chars().count() <= cell {
            ruler.push_str(&" ".repeat(cell - ruler.chars().count()));
            ruler.push_str(&label);
        }
        col += if step == 2 { 3 } else { 6 };
    }
    let ruler: String = ruler.chars().take(hours * step).collect();
    vec![
        Line::from(spark),
        Line::from(Span::styled(ruler, Style::default().fg(theme.dim))),
    ]
}

pub fn render(f: &mut Frame, area: Rect, theme: &Theme) {
    if area.height == 0 || area.width < 12 {
        return;
    }
    let s = weather::snapshot();
    let w = area.width as usize;

    if !s.ready {
        let y = area.y + area.height.saturating_sub(1) / 2;
        f.render_widget(
            Paragraph::new(Span::styled(
                "fetching weather…",
                Style::default().fg(theme.dim),
            ))
            .alignment(Alignment::Center),
            Rect::new(area.x, y, area.width, 1),
        );
        return;
    }

    let g = glyph(s.condition_code, s.is_day);
    if area.height < 3 {
        let line = Line::from(vec![
            Span::styled(format!("{} ", g), Style::default().fg(theme.accent)),
            Span::styled(
                format!("{:.0}°", s.temp_c),
                Style::default().fg(temp_color(theme, s.temp_c)),
            ),
            Span::styled(
                format!(
                    "  {} · feels {:.0}°",
                    weather::describe(s.condition_code),
                    s.feels_c
                ),
                Style::default().fg(theme.dim),
            ),
        ]);
        f.render_widget(Paragraph::new(line), area);
        return;
    }

    let show_art = area.height >= 5 && w >= 38;
    let art_w: u16 = if show_art { 11 } else { 0 };
    let text_w = w.saturating_sub(art_w as usize);
    let mut details = detail_lines(&s, theme, text_w);
    if !show_art {
        // Lead the headline with the glyph when there's no art.
        details[0].spans.insert(
            0,
            Span::styled(format!("{} ", g), Style::default().fg(theme.accent)),
        );
    }
    let forecast = forecast_lines(&s, theme, w);
    let top_h = if show_art { 5 } else { details.len() as u16 };
    let want_forecast = !forecast.is_empty() && area.height >= top_h + 1 + forecast.len() as u16;
    let total = top_h
        + if want_forecast {
            1 + forecast.len() as u16
        } else {
            0
        };
    let y0 = area.y + area.height.saturating_sub(total) / 2;

    if show_art {
        let color = if s.is_day {
            theme.yellow
        } else {
            theme.secondary
        };
        let art_lines: Vec<Line> = art(s.condition_code, s.is_day)
            .iter()
            .map(|r| Line::from(Span::styled(*r, Style::default().fg(color))))
            .collect();
        f.render_widget(
            Paragraph::new(art_lines),
            Rect::new(area.x, y0, art_w.min(area.width), 5),
        );
        let dy = (5u16.saturating_sub(details.len() as u16)) / 2;
        f.render_widget(
            Paragraph::new(details),
            Rect::new(
                area.x + art_w,
                y0 + dy,
                area.width.saturating_sub(art_w),
                5 - dy,
            ),
        );
    } else {
        f.render_widget(
            Paragraph::new(details),
            Rect::new(area.x, y0, area.width, top_h.min(area.height)),
        );
    }

    if want_forecast {
        let fy = y0 + top_h + 1;
        f.render_widget(
            Paragraph::new(forecast),
            Rect::new(area.x, fy, area.width, 2.min(area.y + area.height - fy)),
        );
    }
}
