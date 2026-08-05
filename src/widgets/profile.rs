use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use image::{imageops::FilterType, GenericImageView};

use crate::app;

// Cache the ASCII rendering so we don't recalculate it every frame
type AsciiCache = HashMap<(u16, u16), Vec<Line<'static>>>;
static ASCII_CACHE: OnceLock<Mutex<AsciiCache>> = OnceLock::new();

/// User-configured image path, set once at startup from config. Empty = embedded logo.
static IMAGE_PATH: OnceLock<String> = OnceLock::new();

/// Set the profile image path from config. Call once at startup.
pub fn set_image_path(path: String) {
    let _ = IMAGE_PATH.set(path);
}

fn image_path() -> &'static str {
    IMAGE_PATH.get().map(|s| s.as_str()).unwrap_or("")
}

const ASCII_CHARS: &[u8] = b" .:-=+*#%@";

fn get_char_for_luma(luma: u8) -> char {
    let idx = (luma as usize * (ASCII_CHARS.len() - 1)) / 255;
    ASCII_CHARS[idx] as char
}

const WORDMARK: [&str; 4] = [
    " ██╗   ██╗ █████╗ ███╗   ██╗████████╗ █████╗ ",
    " ██║   ██║██╔══██╗████╗  ██║╚══██╔══╝██╔══██╗",
    " ██║   ██║███████║██╔██╗ ██║   ██║   ███████║",
    " ╚██████╔╝██╔══██║██║ ╚███║   ██║   ██╔══██║",
];

fn fallback_art() -> Vec<Line<'static>> {
    WORDMARK
        .iter()
        .map(|line| Line::from(Span::raw(*line)))
        .collect()
}

/// Styled "VANTA" block wordmark — crisp accent-colored art for the neofetch hero,
/// where photo-ASCII of the logo renders illegibly small.
pub(crate) fn wordmark(theme: &app::Theme) -> Vec<Line<'static>> {
    WORDMARK
        .iter()
        .map(|line| {
            Line::from(Span::styled(
                *line,
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(ratatui::style::Modifier::BOLD),
            ))
        })
        .collect()
}

fn expand_tilde(path: &str) -> String {
    match path.strip_prefix("~/") {
        Some(rest) => match std::env::var("HOME") {
            Ok(home) => format!("{home}/{rest}"),
            Err(_) => path.to_string(),
        },
        None => path.to_string(),
    }
}

fn generate_ascii_art(width: u16, height: u16, image_path: &str) -> Vec<Line<'static>> {
    // Load from a user-configured image path if set, else the embedded logo.
    let img = if !image_path.is_empty() {
        match image::open(expand_tilde(image_path)) {
            Ok(img) => img,
            Err(_) => return fallback_art(),
        }
    } else {
        let img_bytes = include_bytes!("../../assets/logo.png");
        match image::load_from_memory(img_bytes) {
            Ok(img) => img,
            Err(_) => return fallback_art(),
        }
    };

    let target_w = width.max(1) as u32;
    let target_h = height.max(1) as u32;

    let aspect_ratio = img.width() as f32 / img.height() as f32;
    // Terminal characters are roughly twice as tall as they are wide.
    let mut calc_w = target_w;
    let mut calc_h = (calc_w as f32 / (aspect_ratio * 2.1)) as u32; // 2.1 is a good general terminal font aspect

    if calc_h > target_h {
        calc_h = target_h;
        calc_w = (calc_h as f32 * (aspect_ratio * 2.1)) as u32;
    }

    calc_w = calc_w.max(1);
    calc_h = calc_h.max(1);

    let resized = img.resize_exact(calc_w, calc_h, FilterType::Triangle);

    let mut lines = Vec::new();

    for y in 0..resized.height() {
        let mut spans = Vec::new();
        for x in 0..resized.width() {
            let pixel = resized.get_pixel(x, y);
            let r = pixel[0];
            let g = pixel[1];
            let b = pixel[2];
            let a = pixel[3];

            if a < 50 {
                spans.push(Span::raw(" "));
            } else {
                let luma = (r as f32 * 0.299 + g as f32 * 0.587 + b as f32 * 0.114) as u8;
                let ch = get_char_for_luma(luma);
                spans.push(Span::styled(
                    ch.to_string(),
                    Style::default().fg(Color::Rgb(r, g, b)),
                ));
            }
        }
        lines.push(Line::from(spans));
    }
    lines
}



pub fn render(f: &mut Frame, area: Rect, theme: &app::Theme) {
    if area.height < 2 {
        return;
    }

    let mut lines = Vec::new();

    let cache = ASCII_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    let mut map = cache.lock().unwrap();
    let dims = (area.width, area.height);

    // Reserve rows below the logo for the user card.
    let card_rows = 5; // user@host, blank, shell, home, session
    let logo_max_h = area.height.saturating_sub(card_rows).max(2);
    let art_lines = map
        .entry(dims)
        .or_insert_with(|| generate_ascii_art(area.width, logo_max_h, image_path()));

    let username = std::env::var("USER").unwrap_or_else(|_| "vanta".to_string());
    let hostname =
        std::fs::read_to_string("/etc/hostname").unwrap_or_else(|_| "system".to_string());
    let shell = std::env::var("SHELL")
        .ok()
        .and_then(|s| s.rsplit('/').next().map(|s| s.to_string()))
        .unwrap_or_default();
    let home = std::env::var("HOME").unwrap_or_default();
    let session = std::env::var("XDG_SESSION_TYPE").unwrap_or_default();

    // User card below the logo
    let mut card: Vec<Line<'static>> = Vec::new();
    card.push(Line::from(vec![
        Span::styled(
            format!("{}@", username.trim()),
            Style::default().fg(theme.green),
        ),
        Span::styled(
            hostname.trim().to_string(),
            Style::default().fg(theme.accent),
        ),
    ]));
    card.push(Line::from(""));
    for (key, value) in [
        ("shell", shell.clone()),
        ("home", home.clone()),
        ("session", session.clone()),
    ] {
        if value.is_empty() {
            continue;
        }
        card.push(Line::from(vec![
            Span::styled(format!("{:>7} ", key), Style::default().fg(theme.dim)),
            Span::styled(value, Style::default().fg(theme.text)),
        ]));
    }

    // Add vertical padding to center if area is tall enough
    let logo_height = art_lines.len();
    let total_height = logo_height + card.len();

    if area.height as usize > total_height {
        let pad = (area.height as usize - total_height) / 2;
        for _ in 0..pad {
            lines.push(Line::from(""));
        }
    }

    for line in art_lines.iter() {
        lines.push(line.clone());
    }

    lines.extend(card);

    let p = Paragraph::new(lines).alignment(Alignment::Center);
    f.render_widget(p, area);
}
