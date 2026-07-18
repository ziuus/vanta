use ratatui::layout::{Alignment, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::app;

const LOGO: &[&str] = &[
    r#"  ██████           ██████ "#,
    r#"   ██████         ██████  "#,
    r#"    █████        ██████   "#,
    r#"     ███▀▀▀     ██████    "#,
    r#"      ███      ██████     "#,
    r#"       ███    ██████      "#,
    r#"        ███  ██████       "#,
    r#"         █████████        "#,
    r#"          ███████         "#,
    r#"           █████          "#,
];

pub fn render(f: &mut Frame, area: Rect, theme: &app::Theme) {
    if area.height < 6 {
        return;
    }
    
    let mut lines = Vec::new();
    
    let username = std::env::var("USER").unwrap_or_else(|_| "vanta".to_string());
    let hostname = std::fs::read_to_string("/etc/hostname").unwrap_or_else(|_| "system".to_string());
    
    // Add vertical padding to center if area is tall enough
    let logo_height = LOGO.len();
    let text_height = 2; // Username line + spacing
    let total_height = logo_height + text_height;
    
    if area.height as usize > total_height {
        let pad = (area.height as usize - total_height) / 2;
        for _ in 0..pad {
            lines.push(Line::from(""));
        }
    }

    for row in LOGO {
        // Use a neon purple/blue style for the logo to match the sleek design
        lines.push(Line::from(Span::styled(*row, Style::default().fg(theme.secondary))));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled(format!("{}@", username.trim()), Style::default().fg(theme.green)),
        Span::styled(hostname.trim().to_string(), Style::default().fg(theme.accent)),
    ]));

    let p = Paragraph::new(lines).alignment(Alignment::Center);
    f.render_widget(p, area);
}
