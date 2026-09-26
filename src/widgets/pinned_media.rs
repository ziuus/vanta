use ratatui::layout::Alignment;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;
use std::cell::RefCell;

use crate::theme::Theme;

thread_local! {
    static CACHED_IMAGE: RefCell<Option<CachedMedia>> = const { RefCell::new(None) };
}

struct CachedMedia {
    path: String,
    area_width: u16,
    area_height: u16,
    lines: Vec<Line<'static>>,

    // Slideshow state
    is_dir: bool,
    images: Vec<String>,
    current_idx: usize,
    last_tick: u64,
}

pub fn render(f: &mut Frame, area: Rect, theme: &Theme, path: &str, tick: u64) {
    // If path is empty, we will use the built-in default image
    let actual_path = if path.is_empty() {
        "default_fallback"
    } else {
        path
    };

    if area.width < 10 || area.height < 5 {
        return;
    }

    let needs_update = CACHED_IMAGE.with(|c| {
        let cache = c.borrow();
        if cache.is_none() {
            return true;
        }
        let cache = cache.as_ref().unwrap();
        if cache.path != actual_path
            || cache.area_width != area.width
            || cache.area_height != area.height
        {
            return true;
        }
        if cache.is_dir {
            // switch every ~150 ticks (5 sec at 30fps)
            if tick > cache.last_tick + 150 {
                return true;
            }
        }
        false
    });

    if needs_update {
        let expanded_path = if let Some(stripped) = path.strip_prefix("~/") {
            let home = std::env::var("HOME").unwrap_or_else(|_| "".to_string());
            format!("{}/{}", home, stripped)
        } else if path == "~" {
            std::env::var("HOME").unwrap_or_else(|_| "".to_string())
        } else {
            path.to_string()
        };

        let path_buf = std::path::Path::new(&expanded_path);
        let mut is_dir = false;
        let mut images = Vec::new();
        let mut current_idx = 0;
        let mut file_to_load = expanded_path.clone();

        CACHED_IMAGE.with(|c| {
            if let Some(cache) = c.borrow().as_ref() {
                if cache.path == actual_path
                    && cache.area_width == area.width
                    && cache.area_height == area.height
                {
                    is_dir = cache.is_dir;
                    images = cache.images.clone();
                    current_idx = cache.current_idx;
                }
            }
        });

        if path_buf.is_dir() {
            is_dir = true;
            if images.is_empty() {
                if let Ok(entries) = std::fs::read_dir(path_buf) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if path.is_file() {
                            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                                let ext = ext.to_lowercase();
                                if ext == "png"
                                    || ext == "jpg"
                                    || ext == "jpeg"
                                    || ext == "gif"
                                    || ext == "webp"
                                {
                                    images.push(path.to_string_lossy().to_string());
                                }
                            }
                        }
                    }
                }
                images.sort();
            }
            if !images.is_empty() {
                current_idx = (current_idx + 1) % images.len();
                file_to_load = images[current_idx].clone();
            }
        }

        let mut lines = Vec::new();
        let load_result = if actual_path == "default_fallback" {
            image::load_from_memory(include_bytes!("../assets/default_media.jpg"))
        } else {
            image::open(&file_to_load)
        };

        match load_result {
            Ok(img) => {
                let img = img.to_rgb8();
                let target_w = area.width as u32;
                let target_h = (area.height * 2) as u32;

                let thumb = image::imageops::thumbnail(&img, target_w, target_h);
                let (w, h) = thumb.dimensions();

                for y in (0..h).step_by(2) {
                    let mut spans = Vec::new();
                    for x in 0..w {
                        let top = thumb.get_pixel(x, y);
                        let bottom = if y + 1 < h {
                            Some(thumb.get_pixel(x, y + 1))
                        } else {
                            None
                        };

                        let top_color = Color::Rgb(top[0], top[1], top[2]);
                        let style = ratatui::style::Style::default().fg(top_color);
                        let style = if let Some(b) = bottom {
                            let bottom_color = Color::Rgb(b[0], b[1], b[2]);
                            style.bg(bottom_color)
                        } else {
                            style
                        };
                        spans.push(Span::styled("▀", style));
                    }
                    lines.push(Line::from(spans));
                }
            }
            Err(e) => {
                lines = vec![Line::from(vec![Span::styled(
                    format!("Failed to load {}: {}", file_to_load, e),
                    ratatui::style::Style::default().fg(theme.red),
                )])];
            }
        }

        CACHED_IMAGE.with(|c| {
            *c.borrow_mut() = Some(CachedMedia {
                path: path.to_string(),
                area_width: area.width,
                area_height: area.height,
                lines,
                is_dir,
                images,
                current_idx,
                last_tick: tick,
            });
        });
    }

    CACHED_IMAGE.with(|c| {
        if let Some(cache) = c.borrow().as_ref() {
            let p = Paragraph::new(cache.lines.clone()).alignment(Alignment::Center);
            f.render_widget(p, area);
        }
    });
}
