use std::cell::RefCell;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;
use ratatui::layout::Alignment;

use crate::theme::Theme;

thread_local! {
    static CACHED_IMAGE: RefCell<Option<CachedMedia>> = RefCell::new(None);
}

struct CachedMedia {
    path: String,
    area_width: u16,
    area_height: u16,
    lines: Vec<Line<'static>>,
}

pub fn render(f: &mut Frame, area: Rect, theme: &Theme, path: &str) {
    if path.is_empty() {
        let msg = Paragraph::new(vec![
            Line::from(vec![Span::styled(
                "no media pinned",
                ratatui::style::Style::default().fg(theme.dim),
            )]),
            Line::from(vec![Span::styled(
                "set pinned_media_path in config.toml",
                ratatui::style::Style::default().fg(theme.dim),
            )]),
        ])
        .alignment(Alignment::Center);
        f.render_widget(msg, area);
        return;
    }

    if area.width < 10 || area.height < 5 {
        return;
    }

    

    let needs_update = CACHED_IMAGE.with(|c| {
        let cache = c.borrow();
        cache.is_none()
            || cache.as_ref().unwrap().path != path
            || cache.as_ref().unwrap().area_width != area.width
            || cache.as_ref().unwrap().area_height != area.height
    });

    if needs_update {
        let expanded_path = if path.starts_with("~/") {
            let home = std::env::var("HOME").unwrap_or_else(|_| "".to_string());
            format!("{}/{}", home, &path[2..])
        } else if path == "~" {
            std::env::var("HOME").unwrap_or_else(|_| "".to_string())
        } else {
            path.to_string()
        };

        let mut lines = Vec::new();
        match image::open(&expanded_path) {
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
                    format!("Failed to load {}: {}", path, e),
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