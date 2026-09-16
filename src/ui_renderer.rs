use crate::protocol::*;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Gauge, ListItem, List};
use ratatui::Frame;

impl From<UiColor> for Color {
    fn from(color: UiColor) -> Self {
        match color {
            UiColor::Reset => Color::Reset,
            UiColor::Black => Color::Black,
            UiColor::Red => Color::Red,
            UiColor::Green => Color::Green,
            UiColor::Yellow => Color::Yellow,
            UiColor::Blue => Color::Blue,
            UiColor::Magenta => Color::Magenta,
            UiColor::Cyan => Color::Cyan,
            UiColor::Gray => Color::Gray,
            UiColor::DarkGray => Color::DarkGray,
            UiColor::LightRed => Color::LightRed,
            UiColor::LightGreen => Color::LightGreen,
            UiColor::LightYellow => Color::LightYellow,
            UiColor::LightBlue => Color::LightBlue,
            UiColor::LightMagenta => Color::LightMagenta,
            UiColor::LightCyan => Color::LightCyan,
            UiColor::White => Color::White,
            UiColor::Rgb(r, g, b) => Color::Rgb(r, g, b),
        }
    }
}

impl From<UiStyle> for Style {
    fn from(style: UiStyle) -> Self {
        let mut s = Style::default();
        if let Some(c) = style.fg {
            s = s.fg(c.into());
        }
        if let Some(c) = style.bg {
            s = s.bg(c.into());
        }
        if let Some(true) = style.bold {
            s = s.add_modifier(Modifier::BOLD);
        }
        if let Some(true) = style.italic {
            s = s.add_modifier(Modifier::ITALIC);
        }
        if let Some(true) = style.underlined {
            s = s.add_modifier(Modifier::UNDERLINED);
        }
        s
    }
}

pub fn render_widget(widget: &UiWidget, f: &mut Frame, area: Rect) {
    match widget {
        UiWidget::Paragraph { lines, block, wrap } => {
            let text_lines: Vec<Line> = lines.iter().map(|l| {
                let spans: Vec<Span> = l.spans.iter().map(|s| {
                    Span::styled(s.content.clone(), Style::from(s.style.clone().unwrap_or_default()))
                }).collect();
                Line::from(spans)
            }).collect();
            
            let mut p = Paragraph::new(text_lines);
            if *wrap {
                p = p.wrap(ratatui::widgets::Wrap { trim: true });
            }
            if let Some(b) = block {
                let mut blk = Block::default();
                if b.bordered {
                    blk = blk.borders(Borders::ALL);
                }
                if let Some(ref title) = b.title {
                    blk = blk.title(title.clone());
                }
                if let Some(ref color) = b.border_color {
                    blk = blk.border_style(Style::default().fg(color.clone().into()));
                }
                p = p.block(blk);
            }
            f.render_widget(p, area);
        }
        UiWidget::Gauge { ratio, label, block, color } => {
            let mut g = Gauge::default().ratio(*ratio).use_unicode(true);
            if let Some(l) = label {
                g = g.label(l.clone());
            }
            if let Some(c) = color {
                g = g.gauge_style(Style::default().fg(c.clone().into()));
            }
            if let Some(b) = block {
                let mut blk = Block::default();
                if b.bordered {
                    blk = blk.borders(Borders::ALL);
                }
                if let Some(ref title) = b.title {
                    blk = blk.title(title.clone());
                }
                if let Some(ref color) = b.border_color {
                    blk = blk.border_style(Style::default().fg(color.clone().into()));
                }
                g = g.block(blk);
            }
            f.render_widget(g, area);
        }
        UiWidget::List { items, block } => {
            let list_items: Vec<ListItem> = items.iter().map(|l| {
                let spans: Vec<Span> = l.spans.iter().map(|s| {
                    Span::styled(s.content.clone(), Style::from(s.style.clone().unwrap_or_default()))
                }).collect();
                ListItem::new(Line::from(spans))
            }).collect();
            
            let mut lst = List::new(list_items);
            if let Some(b) = block {
                let mut blk = Block::default();
                if b.bordered {
                    blk = blk.borders(Borders::ALL);
                }
                if let Some(ref title) = b.title {
                    blk = blk.title(title.clone());
                }
                if let Some(ref color) = b.border_color {
                    blk = blk.border_style(Style::default().fg(color.clone().into()));
                }
                lst = lst.block(blk);
            }
            f.render_widget(lst, area);
        }
        UiWidget::Column { children, percentages } => {
            let constraints = if let Some(pcts) = percentages {
                pcts.iter().map(|p| Constraint::Percentage(*p)).collect::<Vec<_>>()
            } else {
                let count = children.len();
                if count > 0 {
                    vec![Constraint::Ratio(1, count as u32); count]
                } else {
                    vec![]
                }
            };
            
            let layout = Layout::default()
                .direction(Direction::Vertical)
                .constraints(constraints)
                .split(area);
                
            for (i, child) in children.iter().enumerate() {
                if i < layout.len() {
                    render_widget(child, f, layout[i]);
                }
            }
        }
        UiWidget::Row { children, percentages } => {
            let constraints = if let Some(pcts) = percentages {
                pcts.iter().map(|p| Constraint::Percentage(*p)).collect::<Vec<_>>()
            } else {
                let count = children.len();
                if count > 0 {
                    vec![Constraint::Ratio(1, count as u32); count]
                } else {
                    vec![]
                }
            };
            
            let layout = Layout::default()
                .direction(Direction::Horizontal)
                .constraints(constraints)
                .split(area);
                
            for (i, child) in children.iter().enumerate() {
                if i < layout.len() {
                    render_widget(child, f, layout[i]);
                }
            }
        }
    }
}
