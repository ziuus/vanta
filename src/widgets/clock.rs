use chrono::{Local, Timelike, Utc};
use chrono_tz::Tz;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::theme::Theme;

/// 3×5 glyphs. `#` is lit. Colon is 1 wide.
const GLYPH_H: usize = 5;

fn glyph(c: char, font: &str) -> [&'static str; GLYPH_H] {
    match font {
        "rounded" => match c {
            '0' => [" # ", "# #", "# #", "# #", " # "],
            '1' => ["#", "#", "#", "#", "#"],
            '2' => ["###", "  #", " # ", "#  ", "###"],
            '3' => ["###", "  #", " ##", "  #", "###"],
            '4' => ["# #", "# #", "###", "  #", "  #"],
            '5' => ["###", "#  ", "###", "  #", "###"],
            '6' => [" ##", "#  ", "###", "# #", "###"],
            '7' => ["###", "  #", "  #", "  #", "  #"],
            '8' => [" # ", "# #", " # ", "# #", " # "],
            '9' => ["###", "# #", "###", "  #", " ##"],
            ':' => [" ", "#", " ", "#", " "],
            _ => ["   ", "   ", "   ", "   ", "   "],
        },
        "digital" => match c {
            '0' => ["###", "# #", "# #", "# #", "###"],
            '1' => ["##", " #", " #", " #", "###"],
            '2' => ["###", "  #", "###", "#  ", "###"],
            '3' => ["###", "  #", "###", "  #", "###"],
            '4' => ["# #", "# #", "###", "  #", "  #"],
            '5' => ["###", "#  ", "###", "  #", "###"],
            '6' => ["###", "#  ", "###", "# #", "###"],
            '7' => ["###", "  #", "  #", "  #", "  #"],
            '8' => ["###", "# #", "###", "# #", "###"],
            '9' => ["###", "# #", "###", "  #", "###"],
            ':' => [" ", "#", " ", "#", " "],
            _ => ["   ", "   ", "   ", "   ", "   "],
        },
        _ => match c {
            // standard
            '0' => ["###", "# #", "# #", "# #", "###"],
            '1' => ["##", " #", " #", " #", " #"],
            '2' => ["###", "  #", "###", "#  ", "###"],
            '3' => ["###", "  #", "###", "  #", "###"],
            '4' => ["# #", "# #", "###", "  #", "  #"],
            '5' => ["###", "#  ", "###", "  #", "###"],
            '6' => ["###", "#  ", "###", "# #", "###"],
            '7' => ["###", "  #", "  #", "  #", "  #"],
            '8' => ["###", "# #", "###", "# #", "###"],
            '9' => ["###", "# #", "###", "  #", "###"],
            ':' => [" ", "#", " ", "#", " "],
            _ => ["   ", "   ", "   ", "   ", "   "],
        },
    }
}

/// Width in cells of `text` rendered at horizontal scale `sx`, with one
/// `sx`-wide gap between glyphs.
fn text_width(text: &str, sx: usize, font: &str) -> usize {
    let glyphs = text.chars().count();
    let cells: usize = text.chars().map(|c| glyph(c, font)[0].len()).sum();
    (cells + glyphs.saturating_sub(1)) * sx
}

/// Render `text` as block glyphs. `sx`/`sy` stretch each glyph pixel.
/// `colon_on` toggles the colons so they can blink with the seconds.
fn big_lines(
    clock_style: &str,
    text: &str,
    sx: usize,
    sy: usize,
    colon_on: bool,
    font: &str,
    theme: &Theme,
) -> Vec<Line<'static>> {
    let digit = Style::default().fg(theme.accent);
    let colon = Style::default().fg(if colon_on {
        theme.accent
    } else {
        theme.surface
    });
    let mut out = Vec::with_capacity(GLYPH_H * sy);
    for row in 0..GLYPH_H {
        let mut spans: Vec<Span<'static>> = Vec::new();
        for (i, c) in text.chars().enumerate() {
            if i > 0 {
                spans.push(Span::raw(" ".repeat(sx)));
            }
            let style = if c == ':' { colon } else { digit };
            for px in glyph(c, font)[row].chars() {
                let cell = if px == '#' {
                    match clock_style {
                        "dotted" => "⣿",
                        "hollow" => "▒",
                        _ => "█",
                    }
                } else {
                    " "
                };
                spans.push(Span::styled(cell.repeat(sx), style));
            }
        }
        let line = Line::from(spans);
        for _ in 0..sy {
            out.push(line.clone());
        }
    }
    out
}

/// Pick the largest scale whose glyph block fits. Cells are ~1:2, so sx = 2·sy
/// keeps the 3×5 glyph at its intended proportions; (1,1) is the thin fallback.
fn pick_scale(text: &str, w: usize, h: usize, font: &str) -> Option<(usize, usize)> {
    for &(sx, sy) in &[(6usize, 3usize), (4, 2), (2, 1), (1, 1)] {
        if text_width(text, sx, font) <= w && GLYPH_H * sy <= h {
            return Some((sx, sy));
        }
    }
    None
}

/// Big clock. Layout: block-digit time, then a date line beneath. Falls back
/// to plain text when the area is too small for glyphs.
pub fn render(
    f: &mut Frame,
    area: Rect,
    theme: &Theme,
    h24: bool,
    font: &str,
    style: &str,
    timezones: &[String],
) {
    if area.height == 0 || area.width < 8 {
        return;
    }
    let now = Local::now();
    let date = now.format("%A, %-d %B %Y").to_string();
    let colon_on = now.nanosecond() < 500_000_000;

    let w = area.width as usize;
    // Reserve a row for the date when there's room for glyphs + date.
    // The timezone table only gets rows if the glyphs still fit above it;
    // otherwise the big time wins and the table is simply not drawn.
    let tz_rows = if timezones.is_empty() {
        0
    } else {
        timezones.len() + 2
    };
    let tz_height = if area.height as usize >= GLYPH_H + 2 + tz_rows {
        tz_rows
    } else {
        0
    };
    let glyph_h_avail = (area.height as usize).saturating_sub(2 + tz_height);

    let (full, short) = if h24 {
        (
            now.format("%H:%M:%S").to_string(),
            now.format("%H:%M").to_string(),
        )
    } else {
        (
            now.format("%-I:%M:%S").to_string(),
            now.format("%-I:%M").to_string(),
        )
    };
    let suffix = (!h24).then(|| now.format("%P").to_string());
    let choice = pick_scale(&full, w, glyph_h_avail, font)
        .map(|s| (full.clone(), s, None))
        .or_else(|| {
            pick_scale(&short, w, glyph_h_avail, font)
                .map(|s| (short.clone(), s, Some(now.format("%S").to_string())))
        });

    let Some((text, (sx, sy), seconds)) = choice else {
        // Text fallback for tiny panels.
        let top = area.y + area.height.saturating_sub(2) / 2;
        f.render_widget(
            Paragraph::new(Line::from(Span::styled(
                full,
                Style::default().fg(theme.accent),
            )))
            .alignment(Alignment::Center),
            Rect::new(area.x, top, area.width, 1),
        );
        if area.height >= 2 {
            f.render_widget(
                Paragraph::new(Line::from(Span::styled(
                    date,
                    Style::default().fg(theme.dim),
                )))
                .alignment(Alignment::Center),
                Rect::new(area.x, top + 1, area.width, 1),
            );
        }
        return;
    };

    let mut lines = big_lines(style, &text, sx, sy, colon_on, font, theme);
    // Small seconds (when dropped) and am/pm tag sit at the bottom-right of the glyphs.
    let mut tag = seconds.unwrap_or_default();
    if let Some(s) = &suffix {
        if !tag.is_empty() {
            tag.push(' ');
        }
        tag.push_str(s);
    }
    if !tag.is_empty() {
        if let Some(last) = lines.last_mut() {
            last.spans.push(Span::styled(
                format!(" {}", tag),
                Style::default().fg(theme.dim),
            ));
        }
    }
    let block_h = lines.len() as u16 + 2; // blank + date
    let top = area.y + area.height.saturating_sub(block_h) / 2;
    let glyph_w = text_width(&text, sx, font) as u16;
    let left = area.x + area.width.saturating_sub(glyph_w) / 2;
    let n = lines.len() as u16;
    f.render_widget(
        Paragraph::new(lines),
        Rect::new(left, top, area.width.saturating_sub(left - area.x), n),
    );
    let date_y = top + n + 1;
    if date_y < area.y + area.height {
        f.render_widget(
            Paragraph::new(Line::from(Span::styled(
                date,
                Style::default().fg(theme.dim),
            )))
            .alignment(Alignment::Center),
            Rect::new(area.x, date_y, area.width, 1),
        );
    }

    // Render timezones
    if tz_height > 0 && area.height >= date_y - area.y + 2 + timezones.len() as u16 {
        let mut tz_lines = Vec::new();
        tz_lines.push(Line::from(vec![])); // Spacer
        tz_lines.push(Line::from(vec![
            Span::styled("ZONE", Style::default().fg(theme.dim)),
            Span::raw(" ".repeat(area.width.saturating_sub(12) as usize)),
            Span::styled("TIME", Style::default().fg(theme.dim)),
        ]));

        for tz_str in timezones {
            if let Ok(tz) = tz_str.parse::<Tz>() {
                let time_in_tz = Utc::now().with_timezone(&tz);
                let time_fmt = if h24 {
                    time_in_tz.format("%H:%M:%S")
                } else {
                    time_in_tz.format("%I:%M:%S %p")
                };
                let tz_name = crate::widgets::meter::ellipsize(tz_str, 15);

                let time_str = time_fmt.to_string();
                let pad = area
                    .width
                    .saturating_sub((tz_name.chars().count() + time_str.len()) as u16)
                    as usize;

                tz_lines.push(Line::from(vec![
                    Span::styled(tz_name, Style::default().fg(theme.text)),
                    Span::raw(" ".repeat(pad)),
                    Span::styled(time_str, Style::default().fg(theme.accent)),
                ]));
            }
        }

        f.render_widget(
            Paragraph::new(tz_lines),
            Rect::new(
                area.x,
                date_y + 1,
                area.width,
                area.height.saturating_sub(date_y + 1 - area.y),
            ),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn glyph_widths_and_scale_selection() {
        // '1' has width 2 in standard font: 19 cells + 7 gaps = 26 cells at scale 1.
        assert_eq!(text_width("12:34:56", 1, "standard"), 26);
        assert_eq!(text_width("12:34:56", 2, "standard"), 52);
        assert_eq!(pick_scale("12:34:56", 60, 5, "standard"), Some((2, 1)));
        assert_eq!(pick_scale("12:34:56", 120, 10, "standard"), Some((4, 2)));
        assert_eq!(pick_scale("12:34:56", 30, 5, "standard"), Some((1, 1)));
        assert_eq!(pick_scale("12:34:56", 20, 5, "standard"), None);
        for c in "0123456789:".chars() {
            assert!(glyph(c, "standard")
                .iter()
                .all(|row| row.len() == glyph(c, "standard")[0].len()));
        }
    }
}
