use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

const CHARSET: &str = "ｱｲｳｴｵｶｷｸｹｺｻｼｽｾｿﾀﾁﾂﾃﾄﾅﾆﾇﾈﾉﾊﾋﾌﾍﾎﾏﾐﾑﾒﾓﾔﾕﾖﾗﾘﾙﾚﾛﾜｦﾝ0123456789";

use std::time::{SystemTime, UNIX_EPOCH};

/// Render a Matrix-style falling-characters effect into the given area.
///
/// Uses `SystemTime` as a deterministic seed so the animation advances smoothly
/// at 60fps with no mutable state.
#[allow(clippy::needless_range_loop)]
#[allow(clippy::unnecessary_cast)]
pub fn render(f: &mut Frame, area: Rect, _tick: u64) {
    if area.width == 0 || area.height == 0 {
        return;
    }

    // Derived tick for smooth ~15-20 units per second speed
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u64;
    let tick = now / 60;


    let width = area.width as usize;
    let height = area.height as usize;

    let mut rows: Vec<Vec<(char, u8)>> = vec![vec![(' ', 0); width]; height];
    let matrix_chars: Vec<char> = CHARSET.chars().collect();

    for col in 0..width {
        // Space out columns slightly to look better (every other column is empty in many cmatrix implementations)
        if col % 2 != 0 {
            continue;
        }
        
        let col_seed = col.wrapping_mul(713).wrapping_add(12345);
        // speed modifier
        let speed_div = 1 + (col_seed % 3);
        let tick_local = (tick / (speed_div as u64)) as usize;
        
        let interval = 40 + (col_seed % 60); // 40-100 ticks per cycle

        // Draw multiple drops per column to allow density
        for drop_idx in 0..2 {
            let offset_time = tick_local.wrapping_add(drop_idx * (interval / 2)).wrapping_add(col_seed % interval);
            let cycle = offset_time / interval;
            let cycle_seed = col_seed.wrapping_add(cycle.wrapping_mul(997));
            
            let drop_head = offset_time % interval;
            
            // Trail length 10-30
            let trail_len = 10 + (cycle_seed % 20);
            
            for row in 0..trail_len {
                let y = if drop_head >= row {
                    drop_head - row
                } else {
                    continue;
                };

                if y >= height {
                    continue;
                }

                let brightness = if row == 0 {
                    255 // head
                } else if row < 3 {
                    200 // intense trail
                } else {
                    // fade out
                    let ratio = row as f32 / trail_len as f32;
                    (255.0 * (1.0 - ratio)) as u8
                };

                // The character should change based on tick to give the "matrix" changing effect
                let time_offset = (tick / 5) as usize; // change chars every 5 ticks
                let ch_idx = cycle_seed.wrapping_add(row.wrapping_mul(31)).wrapping_add(time_offset)
                    % matrix_chars.len();
                let ch = matrix_chars[ch_idx];

                // If multiple drops overlap, take the brightest
                if brightness > rows[y][col].1 {
                    rows[y][col] = (ch, brightness);
                }
            }
        }
    }

    let mut lines: Vec<Line> = Vec::with_capacity(height);
    for y in 0..height {
        let mut spans = Vec::with_capacity(width);
        for x in 0..width {
            let (ch, brightness) = rows[y][x];
            if brightness == 0 {
                spans.push(Span::raw(" "));
            } else {
                let (r, g, b) = match brightness {
                    255 => (200, 255, 200), // white-ish green head
                    200 => (40, 220, 40),
                    b => (0, (b as u32 * 180 / 255).max(30) as u8, 0),
                };
                spans.push(Span::styled(
                    ch.to_string(),
                    Style::default().fg(Color::Rgb(r, g, b)),
                ));
            }
        }
        lines.push(Line::from(spans));
    }

    f.render_widget(Paragraph::new(lines).style(Style::default().bg(Color::Rgb(0, 0, 0))), area);
}
