use image::{imageops::FilterType, GenericImageView};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};

/// Render an image as half-blocks (upper and lower halves) with truecolor.
///
/// Each terminal cell covers 1x2 source pixels. A `w`x`h` cell area resolves
/// `w`x`2h` pixels. Half-blocks appear much denser and more solid than braille.
pub fn render_image(img: &image::DynamicImage, w: u16, h: u16) -> Vec<Line<'static>> {
    if w == 0 || h == 0 {
        return Vec::new();
    }

    let (iw, ih) = img.dimensions();
    if iw == 0 || ih == 0 {
        return Vec::new();
    }
    
    // Fit image inside the cell box, preserving aspect.
    // Cells are typically ~2x taller than wide, and half-blocks map 1 cell to 1x2 pixels,
    // so in pixel space, the cell aspect ratio is roughly 1:1.
    let aspect = iw as f32 / ih as f32;
    let mut cw = w;
    let mut ch = (cw as f32 / aspect / 2.0).ceil() as u16;
    if ch > h {
        ch = h;
        cw = (ch as f32 * aspect * 2.0).ceil() as u16;
        cw = cw.min(w);
    }
    let (cw, ch) = (cw.max(1), ch.max(1));

    let px_w = cw as u32;
    let px_h = ch as u32 * 2;
    let small = img.resize_exact(px_w, px_h, FilterType::Lanczos3).to_rgba8();

    let mut lines = Vec::with_capacity(ch as usize);
    for cy in 0..ch as u32 {
        let mut spans: Vec<Span<'static>> = Vec::with_capacity(cw as usize);
        for cx in 0..cw as u32 {
            let p_top = small.get_pixel(cx, cy * 2);
            let p_bot = small.get_pixel(cx, cy * 2 + 1);

            let top_vis = p_top[3] >= 50;
            let bot_vis = p_bot[3] >= 50;

            if !top_vis && !bot_vis {
                spans.push(Span::raw(" "));
                continue;
            }

            if top_vis && !bot_vis {
                let color = Color::Rgb(p_top[0], p_top[1], p_top[2]);
                spans.push(Span::styled("▀", Style::default().fg(color)));
            } else if !top_vis && bot_vis {
                let color = Color::Rgb(p_bot[0], p_bot[1], p_bot[2]);
                spans.push(Span::styled("▄", Style::default().fg(color)));
            } else {
                // Both visible. Use an upper half block with foreground as top and background as bottom.
                let fg = Color::Rgb(p_top[0], p_top[1], p_top[2]);
                let bg = Color::Rgb(p_bot[0], p_bot[1], p_bot[2]);
                spans.push(Span::styled("▀", Style::default().fg(fg).bg(bg)));
            }
        }
        lines.push(Line::from(spans));
    }
    lines
}

#[allow(dead_code)]
pub fn render_path(path: &str, w: u16, h: u16) -> Option<Vec<Line<'static>>> {
    let img = image::open(path).ok()?;
    let out = render_image(&img, w, h);
    if out.is_empty() {
        None
    } else {
        Some(out)
    }
}
