use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use super::{Component, Extension, ExtensionMetadata, Page};
use crate::theme::Theme;

/// A mock component for demonstration
pub struct HelloWorldComponent;

impl Component for HelloWorldComponent {
    fn id(&self) -> &str {
        "hello_world"
    }

    fn render(&mut self, f: &mut Frame, area: Rect, theme: &Theme) {
        let block = Block::default()
            .borders(Borders::ALL)
            .title(Span::styled(
                " Hello World Widget ",
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            ))
            .border_style(Style::default().fg(theme.dim));

        let content = Paragraph::new(vec![
            Line::from(vec![Span::raw(" This is a custom component!")]),
            Line::from(vec![Span::styled(
                " It was loaded from an extension.",
                Style::default().fg(theme.green),
            )]),
        ])
        .block(block);

        f.render_widget(content, area);
    }
}

/// A mock component for demonstration
pub struct ServerPingComponent;

impl Component for ServerPingComponent {
    fn id(&self) -> &str {
        "server_ping"
    }

    fn render(&mut self, f: &mut Frame, area: Rect, theme: &Theme) {
        let block = Block::default()
            .borders(Borders::ALL)
            .title(Span::styled(
                " Server Ping ",
                Style::default().fg(theme.red).add_modifier(Modifier::BOLD),
            ))
            .border_style(Style::default().fg(theme.dim));

        let content = Paragraph::new(vec![
            Line::from(vec![
                Span::styled(
                    "us-east-1",
                    Style::default()
                        .fg(theme.green)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(" 12ms"),
            ]),
            Line::from(vec![
                Span::styled("eu-central", Style::default().fg(theme.yellow)),
                Span::raw(" 145ms"),
            ]),
        ])
        .block(block);

        f.render_widget(content, area);
    }
}

/// The Template Page bringing it all together
pub struct TemplatePage {
    hello: HelloWorldComponent,
    ping: ServerPingComponent,
}

impl Default for TemplatePage {
    fn default() -> Self {
        Self::new()
    }
}

impl TemplatePage {
    pub fn new() -> Self {
        Self {
            hello: HelloWorldComponent,
            ping: ServerPingComponent,
        }
    }
}

impl Page for TemplatePage {
    fn id(&self) -> &str {
        "template_dashboard"
    }

    fn title(&self) -> &'static str {
        "Template"
    }

    fn render(&mut self, f: &mut Frame, area: Rect, theme: &Theme) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
            .split(area);

        self.hello.render(f, chunks[0], theme);
        self.ping.render(f, chunks[1], theme);
    }
}

/// The main Template Extension
pub struct TemplateExtension;

impl Extension for TemplateExtension {
    fn metadata(&self) -> ExtensionMetadata {
        ExtensionMetadata {
            id: "template".to_string(),
            name: "Vanta Template Extension".to_string(),
            author: "Vanta Core".to_string(),
            version: "1.0.0".to_string(),
            api_version: "0.9.0".to_string(),
            description: "A reference implementation of a Vanta Extension.".to_string(),
        }
    }

    fn pages(&self) -> Vec<Box<dyn Page>> {
        vec![Box::new(TemplatePage::new())]
    }

    fn components(&self) -> Vec<Box<dyn Component>> {
        vec![Box::new(HelloWorldComponent), Box::new(ServerPingComponent)]
    }
}
