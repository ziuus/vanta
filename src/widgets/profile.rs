use ratatui::layout::{Alignment, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::app;

const LOGO: &[&str] = &[
    r#"                     "#,
    r#"                     "#,
    r#"  _    __            __        "#,
    r#" | |  / /___ _____  / /_____ _ "#,
    r#" | | / / __ `/ __ \/ __/ __ `/ "#,
    r#" | |/ / /_/ / / / / /_/ /_/ /  "#,
    r#" |___/\__,_/_/ /_/\__/\__,_/   "#,
    r#"                     "#,
];

pub fn render(f: &mut Frame, area: Rect, theme: &app::Theme) {
    if area.height < 6 {
        return;
    }
    
    let mut lines = Vec::new();
    
    let username = std::env::var("USER").unwrap_or_else(|_| "vanta".to_string());
    let hostname = std::fs::read_to_string("/etc/hostname").unwrap_or_else(|_| "system".to_string());
    
    for row in LOGO {
        lines.push(Line::from(Span::styled(*row, Style::default().fg(theme.focus))));
    }
    lines.push(Line::from(vec![
        Span::styled(format!("{}@", username.trim()), Style::default().fg(theme.green)),
        Span::styled(hostname.trim().to_string(), Style::default().fg(theme.accent)),
    ]));

    let p = Paragraph::new(lines).alignment(Alignment::Center);
    f.render_widget(p, area);
}
