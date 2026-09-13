use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;

use crate::app::App;
use crate::monitors::obsidian;

pub fn render(f: &mut Frame, area: Rect, app: &mut App) {
    let snap = obsidian::snapshot();
    let theme = &app.theme;

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(30),
            Constraint::Percentage(70),
        ])
        .spacing(2)
        .split(area);

    let list_area = chunks[0];
    let content_area = chunks[1];

    let mut list_lines = Vec::new();
    
    // Header for list
    list_lines.push(Line::from(vec![
        Span::styled(" RECENT NOTES", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
    ]));
    list_lines.push(Line::from(""));

    let num_notes = snap.notes.len();
    if num_notes == 0 {
        list_lines.push(Line::from(vec![
            Span::styled(" No notes found in vault.", Style::default().fg(theme.dim)),
        ]));
    } else {
        // Fix up the selected index
        let max_idx = num_notes.saturating_sub(1);
        app.panel_states.obsidian_selected = app.panel_states.obsidian_selected.min(max_idx);
        let selected = app.panel_states.obsidian_selected;

        for (i, note) in snap.notes.iter().enumerate() {
            if i == selected {
                list_lines.push(Line::from(vec![
                    Span::styled(" > ", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
                    Span::styled(&note.title, Style::default().fg(theme.text).add_modifier(Modifier::BOLD)),
                ]));
            } else {
                list_lines.push(Line::from(vec![
                    Span::raw("   "),
                    Span::styled(&note.title, Style::default().fg(theme.dim)),
                ]));
            }
            list_lines.push(Line::from("")); // Spacious
        }
    }

    f.render_widget(
        Paragraph::new(list_lines)
            .block(Block::default().borders(Borders::RIGHT).border_style(Style::default().fg(theme.surface))),
        list_area,
    );

    // Content area
    let mut content_lines = Vec::new();
    if num_notes > 0 {
        let selected = app.panel_states.obsidian_selected;
        let note = &snap.notes[selected];

        content_lines.push(Line::from(vec![
            Span::styled(&note.title, Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
        ]));
        content_lines.push(Line::from(""));
        
        for line in note.content.lines() {
            content_lines.push(Line::from(Span::styled(line, Style::default().fg(theme.text))));
        }
    }

    f.render_widget(
        Paragraph::new(content_lines).wrap(Wrap { trim: false }),
        content_area,
    );
}
