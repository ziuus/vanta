use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;

use crate::monitors::files::{self, PreviewContent};
use ratatui::style::Color;

pub fn render(
    f: &mut Frame,
    area: Rect,
    theme: &crate::theme::Theme,
    is_focused: bool,
    selected_idx: &mut usize,
    scroll: &mut usize,
) {
    let snap = files::snapshot();

    // Split 40% list, 60% preview
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .spacing(1)
        .split(area);

    let list_area = chunks[0];
    let preview_area = chunks[1];

    
    let num_items = snap.items.len();

    // Adjust selected index
    let max_idx = num_items.saturating_sub(1);
    *selected_idx = (*selected_idx).min(max_idx);
    let selected = *selected_idx;

    let list_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(2), Constraint::Min(0)])
        .split(list_area);
        
    let cur_dir = snap.current_dir.to_string_lossy();
    let header = Paragraph::new(vec![
        Line::from(vec![Span::styled(
            format!(" 📁 {}", cur_dir),
            Style::default().fg(theme.accent),
        )]),
        Line::from(""),
    ]);
    
    let border_color = if is_focused {
        theme.accent
    } else {
        theme.surface
    };

    let list_block = Block::default()
        .borders(Borders::RIGHT)
        .border_style(Style::default().fg(border_color));
        
    // Apply the block to the whole list_area, but we have to render it first and then render inner chunks
    let inner_list_area = list_block.inner(list_area);
    f.render_widget(list_block, list_area);
    
    let list_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(2), Constraint::Min(0)])
        .split(inner_list_area);

    f.render_widget(header, list_chunks[0]);

    let visible_items = list_chunks[1].height as usize;
    if selected < *scroll {
        *scroll = selected;
    } else if selected >= *scroll + visible_items && visible_items > 0 {
        *scroll = selected.saturating_sub(visible_items - 1);
    }
    
    
    let mut list_lines = Vec::new();
    for (i, item) in snap.items.iter().enumerate().skip(*scroll).take(visible_items) {
        let prefix = if i == selected { " > " } else { "   " };
        let icon = if item.is_dir { "📁" } else { "📄" };
        let style = if i == selected {
            Style::default().fg(theme.accent)
        } else if item.is_dir {
            Style::default().fg(theme.text)
        } else {
            Style::default().fg(theme.dim)
        };
        list_lines.push(Line::from(vec![
            Span::styled(prefix, Style::default().fg(theme.accent)),
            Span::styled(format!("{} ", icon), style),
            Span::styled(&item.name, style),
        ]));
    }
    
    f.render_widget(Paragraph::new(list_lines), list_chunks[1]);


    // Preview
    let mut preview_lines = Vec::new();
    if num_items > 0 {
        if let Some(preview) = &snap.preview {
            match preview {
                PreviewContent::Text(text) => {
                    for line in text.lines() {
                        preview_lines.push(Line::from(Span::styled(
                            line,
                            Style::default().fg(theme.text),
                        )));
                    }
                }
                PreviewContent::Image {
                    width,
                    height,
                    pixels,
                } => {
                    let w = *width as usize;
                    let h = *height as usize;

                    // Display image info
                    preview_lines.push(Line::from(Span::styled(
                        format!("Image {}x{}", w, h),
                        Style::default().fg(theme.dim),
                    )));
                    preview_lines.push(Line::from(""));

                    // We need to scale it to fit the preview_area
                    let term_w = preview_area.width.saturating_sub(4) as usize;
                    let term_h = (preview_area.height.saturating_sub(4) * 2) as usize; // 2 pixels per char height

                    if term_w > 0 && term_h > 0 && w > 0 && h > 0 {
                        let scale = (w as f32 / term_w as f32)
                            .max(h as f32 / term_h as f32)
                            .max(1.0);
                        let out_w = (w as f32 / scale) as usize;
                        let out_h = (h as f32 / scale) as usize;

                        let get_px = |x: usize, y: usize| -> (u8, u8, u8) {
                            let src_x = (x as f32 * scale).min((w - 1) as f32) as usize;
                            let src_y = (y as f32 * scale).min((h - 1) as f32) as usize;
                            pixels[src_y * w + src_x]
                        };

                        // Render using half-blocks: top is fg, bottom is bg
                        for y in (0..out_h).step_by(2) {
                            let mut spans = Vec::with_capacity(out_w);
                            for x in 0..out_w {
                                let top = get_px(x, y);
                                let bot = if y + 1 < out_h {
                                    get_px(x, y + 1)
                                } else {
                                    (0, 0, 0)
                                };

                                let fg = Color::Rgb(top.0, top.1, top.2);
                                let bg = if y + 1 < out_h {
                                    Color::Rgb(bot.0, bot.1, bot.2)
                                } else {
                                    theme.bg
                                };

                                spans.push(Span::styled("▀", Style::default().fg(fg).bg(bg)));
                            }
                            preview_lines.push(Line::from(spans));
                        }
                    }
                }
            }
        }
    }

    f.render_widget(
        Paragraph::new(preview_lines).wrap(Wrap { trim: false }),
        preview_area,
    );
}
