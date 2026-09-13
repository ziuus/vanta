use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;

use crate::app::{App, PanelId};
use crate::monitors::obsidian;
use crate::screens::panel_full;

pub fn render(f: &mut Frame, area: Rect, app: &mut App) {
    let snap = obsidian::snapshot();
    let theme = &app.theme;
    let cfg = &app.config.widgets;
    let focus = |id: PanelId| app.focused_panel == Some(id);

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(25), // Productivity (Tasks, Agenda, News)
            Constraint::Percentage(25), // Notes List
            Constraint::Percentage(50), // Notes Content
        ])
        .spacing(2)
        .split(area);

    let prod_area = chunks[0];
    let list_area = chunks[1];
    let content_area = chunks[2];

    // Left Productivity Column
    let rows = Layout::vertical([
        Constraint::Length(if cfg.agenda { 10 } else { 0 }),
        Constraint::Length(if cfg.tasks { 12 } else { 0 }),
        Constraint::Min(8), // News takes the rest
    ]).spacing(1).split(prod_area);

    if cfg.agenda {
        let snap = crate::monitors::agenda::snapshot();
        let count = snap.events.len();
        let agenda_rt = if count == 0 { " no events ".to_string() } else { format!(" {} upcoming ", count) };
        let inner = panel_full(f, rows[0], "agenda", Some(&agenda_rt), None, theme, focus(PanelId::Agenda));
        crate::widgets::agenda::render(f, inner, theme);
    }
    
    if cfg.tasks {
        let snap = crate::monitors::tasks::snapshot();
        let open = snap.tasks.iter().filter(|t| !t.completed).count();
        let tasks_rt = format!(" {} open ", open);
        let inner = panel_full(f, rows[1], "tasks", Some(&tasks_rt), None, theme, focus(PanelId::Tasks));
        crate::widgets::tasks::render(f, inner, theme);
    }

    if cfg.news {
        let snap = crate::monitors::news::snapshot();
        let source = if snap.channel_title.is_empty() { " fetching ".to_string() } else { format!(" {} ", snap.channel_title) };
        let inner = panel_full(f, rows[2], "news", Some(&source), None, theme, focus(PanelId::News));
        crate::widgets::news::render(f, inner, theme);
    }

    // Middle Notes List Column
    let mut list_lines = Vec::new();
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
        let max_idx = num_notes.saturating_sub(1);
        app.panel_states.writer_selected = app.panel_states.writer_selected.min(max_idx);
        let selected = app.panel_states.writer_selected;

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
            list_lines.push(Line::from(""));
        }
    }

    let is_focused = focus(PanelId::WriterNotes);
    let border_color = if is_focused { theme.accent } else { theme.surface };
    f.render_widget(
        Paragraph::new(list_lines)
            .block(Block::default().borders(Borders::RIGHT).border_style(Style::default().fg(border_color))),
        list_area,
    );

    // Right Content Column
    let mut content_lines = Vec::new();
    if num_notes > 0 {
        let selected = app.panel_states.writer_selected;
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