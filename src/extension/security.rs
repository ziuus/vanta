use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::theme::Theme;
use super::{Component, Extension, ExtensionMetadata, Page};

/// A mock component for ClamAV status
pub struct ClamAvComponent;

impl Component for ClamAvComponent {
    fn id(&self) -> &'static str {
        "clamav"
    }

    fn render(&mut self, f: &mut Frame, area: Rect, theme: &Theme) {
        let block = Block::default()
            .borders(Borders::ALL)
            .title(Span::styled(" ClamAV Status ", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)))
            .border_style(Style::default().fg(theme.dim));
        
        let content = Paragraph::new(vec![
            Line::from(vec![Span::raw(" Engine: "), Span::styled("Online", Style::default().fg(theme.green))]),
            Line::from(vec![Span::raw(" Signatures: "), Span::raw("8,642,763 (3d old)")]),
            Line::from(vec![Span::raw(" Last Scan: "), Span::raw("0 threats found.")]),
        ]).block(block);
        
        f.render_widget(content, area);
    }
}

/// A mock component for CVE Feed
pub struct CveFeedComponent;

impl Component for CveFeedComponent {
    fn id(&self) -> &'static str {
        "cve_feed"
    }

    fn render(&mut self, f: &mut Frame, area: Rect, theme: &Theme) {
        let block = Block::default()
            .borders(Borders::ALL)
            .title(Span::styled(" CVE Feed (Live) ", Style::default().fg(theme.red).add_modifier(Modifier::BOLD)))
            .border_style(Style::default().fg(theme.dim));
        
        let content = Paragraph::new(vec![
            Line::from(vec![Span::styled("CVE-2026-77179", Style::default().fg(theme.red).add_modifier(Modifier::BOLD)), Span::raw(" 9.4 CRIT Docker Sandboxes")]),
            Line::from(vec![Span::styled("CVE-2026-16141", Style::default().fg(theme.yellow)), Span::raw(" 8.1 HIGH OpenBMC phosphor-net-ipmid")]),
            Line::from(vec![Span::raw("CVE-2026-92079 ??? Mozilla Firefox")]),
        ]).block(block);
        
        f.render_widget(content, area);
    }
}

/// The Security Page bringing it all together
pub struct SecurityPage {
    clamav: ClamAvComponent,
    cve: CveFeedComponent,
}

impl SecurityPage {
    pub fn new() -> Self {
        Self {
            clamav: ClamAvComponent,
            cve: CveFeedComponent,
        }
    }
}

impl Page for SecurityPage {
    fn id(&self) -> &'static str {
        "security_dashboard"
    }

    fn title(&self) -> &'static str {
        "Security"
    }

    fn render(&mut self, f: &mut Frame, area: Rect, theme: &Theme) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
            .split(area);

        self.cve.render(f, chunks[0], theme);
        self.clamav.render(f, chunks[1], theme);
    }
}

/// The main Security Extension
pub struct SecurityExtension;

impl Extension for SecurityExtension {
    fn metadata(&self) -> ExtensionMetadata {
        ExtensionMetadata {
            id: "security",
            name: "Vanta Security (Community Edition)",
            author: "cybermaksx (ported by Vanta core)",
            version: "1.0.0",
            description: "A community extension adding CVE feeds and ClamAV status.",
        }
    }

    fn pages(&self) -> Vec<Box<dyn Page>> {
        vec![Box::new(SecurityPage::new())]
    }

    fn components(&self) -> Vec<Box<dyn Component>> {
        vec![
            Box::new(ClamAvComponent),
            Box::new(CveFeedComponent),
        ]
    }
}
