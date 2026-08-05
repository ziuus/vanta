use image::{imageops::FilterType, GenericImageView};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};

/// Braille dot bit layout within a 2x4 cell (Unicode U+2800 block):
///
/// ```text
///   col0 col1
///    1    4     bit0 bit3
///    2    5     bit1 bit4
///    3    6     bit2 bit5
///    7    8     bit6 bit7
/// ```
const DOT_BITS: [[u8; 2]; 4] = [[0, 3], [1, 4], [2, 5], [6, 7]];

/// Render an image as braille text at 2x4 subpixel density with truecolor.
///
/// Each terminal cell covers 2x4 source pixels, so a `w`x`h` cell area resolves
/// `2w`x`4h` pixels — 8x the detail of a one-char-per-pixel ASCII ramp.
/// A dot is lit when its pixel is brighter than the block's mean luma, which
/// keeps edges crisp regardless of the image's overall brightness.
pub fn render_image(img: &image::DynamicImage, w: u16, h: u16) -> Vec<Line<'static>> {
    if w == 0 || h == 0 {
        return Vec::new();
    }

    // Fit the image inside the cell box, preserving aspect. Cells are ~2x taller
    // than wide, and each holds 2x4 dots, so a cell is ~1:1 in dot space.
    let (iw, ih) = img.dimensions();
    if iw == 0 || ih == 0 {
        return Vec::new();
    }
    let aspect = iw as f32 / ih as f32;
    let mut cw = w;
    let mut ch = (cw as f32 / aspect / 2.0).ceil() as u16;
    if ch > h {
        ch = h;
        cw = (ch as f32 * aspect * 2.0).ceil() as u16;
        cw = cw.min(w);
    }
    let (cw, ch) = (cw.max(1), ch.max(1));

    let px_w = cw as u32 * 2;
    let px_h = ch as u32 * 4;
    let small = img.resize_exact(px_w, px_h, FilterType::Lanczos3).to_rgba8();

    // Image-wide luma midpoint, used as the dot threshold. Computed once over
    // the opaque pixels so solid regions stay solid instead of self-cancelling.
    let (mut sum, mut count) = (0f64, 0u32);
    for p in small.pixels() {
        if p[3] >= 50 {
            sum += p[0] as f64 * 0.299 + p[1] as f64 * 0.587 + p[2] as f64 * 0.114;
            count += 1;
        }
    }
    let global_mid = if count == 0 {
        128.0
    } else {
        (sum / count as f64) as f32
    };

    let mut lines = Vec::with_capacity(ch as usize);
    for cy in 0..ch as u32 {
        let mut spans: Vec<Span<'static>> = Vec::with_capacity(cw as usize);
        for cx in 0..cw as u32 {
            // Gather the 2x4 block: luma for dot decisions, RGB for the cell color.
            let mut luma = [0f32; 8];
            let (mut r, mut g, mut b, mut opaque) = (0u32, 0u32, 0u32, 0u32);
            for (dy, row) in DOT_BITS.iter().enumerate() {
                for (dx, _) in row.iter().enumerate() {
                    let p = small.get_pixel(cx * 2 + dx as u32, cy * 4 + dy as u32);
                    let l = p[0] as f32 * 0.299 + p[1] as f32 * 0.587 + p[2] as f32 * 0.114;
                    luma[dy * 2 + dx] = if p[3] < 50 { -1.0 } else { l };
                    if p[3] >= 50 {
                        r += p[0] as u32;
                        g += p[1] as u32;
                        b += p[2] as u32;
                        opaque += 1;
                    }
                }
            }

            if opaque == 0 {
                spans.push(Span::raw(" "));
                continue;
            }

            // Threshold against the *image-wide* midpoint rather than the block
            // mean: a block that is uniformly bright (or uniformly dark) has no
            // internal edge, and comparing it to its own mean would light half
            // its dots anyway, turning solid regions into ⣿ mush.
            let block_mean: f32 = luma.iter().filter(|l| **l >= 0.0).sum::<f32>() / opaque as f32;
            let thresh = global_mid;

            let mut pattern = 0u8;
            for (dy, row) in DOT_BITS.iter().enumerate() {
                for (dx, bit) in row.iter().enumerate() {
                    let l = luma[dy * 2 + dx];
                    if l >= 0.0 && l >= thresh {
                        pattern |= 1 << bit;
                    }
                }
            }

            // Uniform block that sits above the midpoint: fill it solid so large
            // bright areas stay bright instead of dropping out.
            if pattern == 0 && block_mean >= global_mid {
                pattern = 0xFF;
            }

            if pattern == 0 {
                spans.push(Span::raw(" "));
                continue;
            }

            let ch_out = char::from_u32(0x2800 + pattern as u32).unwrap_or(' ');
            let color = Color::Rgb(
                (r / opaque) as u8,
                (g / opaque) as u8,
                (b / opaque) as u8,
            );
            spans.push(Span::styled(ch_out.to_string(), Style::default().fg(color)));
        }
        lines.push(Line::from(spans));
    }
    lines
}

/// Load an image from disk and render it as braille. `None` if it can't be read.
#[allow(dead_code)] // used by media album art (phase 4)
pub fn render_path(path: &str, w: u16, h: u16) -> Option<Vec<Line<'static>>> {
    let img = image::open(path).ok()?;
    let out = render_image(&img, w, h);
    if out.is_empty() {
        None
    } else {
        Some(out)
    }
}
