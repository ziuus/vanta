use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;

use crate::app::{App, PanelId};
use crate::monitors::obsidian;
use crate::screens::{panel, panel_full};

pub fn render(f: &mut Frame, area: Rect, app: &mut App) {
    let snap = obsidian::snapshot();
    let theme = &app.theme;
    let cfg = &app.config.widgets;
    let focus = |id: PanelId| app.focused_panel == Some(id);

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(app.panel_states.work_ratio), // Productivity
            Constraint::Percentage(100 - app.panel_states.work_ratio), // Notes + Files
        ])
        .spacing(2)
        .split(area);

    let prod_area = chunks[0];
    let right_area = chunks[1];

    let right_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(right_area);

    let notes_area = right_chunks[0];
    let files_area = right_chunks[1];

    // Left Productivity Column
    let rows = Layout::vertical([
        Constraint::Length(if cfg.agenda { 10 } else { 0 }),
        Constraint::Length(if cfg.tasks { 12 } else { 0 }),
        Constraint::Min(8), // News takes the rest
    ])
    .spacing(1)
    .split(prod_area);

    if cfg.agenda {
        let snap = crate::monitors::agenda::snapshot();
        let count = snap.events.len();
        let agenda_rt = if count == 0 {
            " no events ".to_string()
        } else {
            format!(" {} upcoming ", count)
        };
        let inner = panel_full(
            f,
            rows[0],
            "agenda",
            Some(&agenda_rt),
            Some("Enter/e edit"),
            theme,
            focus(PanelId::Agenda),
        );
        crate::widgets::agenda::render(f, inner, theme);
    }

    if cfg.tasks {
        let snap = crate::monitors::tasks::snapshot();
        let open = snap.tasks.iter().filter(|t| !t.completed).count();
        let tasks_rt = format!(" {} open ", open);
        let hint = if app.panel_states.task_input_active {
            "Enter submit · Esc cancel"
        } else {
            "a add · Space toggle · d del · e edit"
        };
        let inner = panel_full(
            f,
            rows[1],
            "tasks",
            Some(&tasks_rt),
            Some(hint),
            theme,
            focus(PanelId::Tasks),
        );
        crate::widgets::tasks::render(
            f,
            inner,
            theme,
            focus(PanelId::Tasks),
            app.panel_states.tasks_selected,
            app.panel_states.task_input_active,
            &app.panel_states.task_input,
        );
    }

    if cfg.news {
        let snap = crate::monitors::news::snapshot();
        let source = if snap.channel_title.is_empty() {
            " fetching ".to_string()
        } else {
            format!(" {} ", snap.channel_title)
        };
        let inner = panel_full(
            f,
            rows[2],
            "news",
            Some(&source),
            None,
            theme,
            focus(PanelId::News),
        );
        crate::widgets::news::render(f, inner, theme);
    }

    // Top Right: Notes
    let notes_title = if snap.vault_name.is_empty() {
        "obsidian (notes)".to_string()
    } else {
        format!("obsidian ({})", snap.vault_name)
    };
    let notes_inner = panel_full(
        f,
        notes_area,
        &notes_title,
        None,
        Some("Enter/e edit · ↑↓ select"),
        theme,
        focus(PanelId::WriterNotes),
    );
    let note_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(33), Constraint::Percentage(67)])
        .spacing(1)
        .split(notes_inner);

    let list_area = note_chunks[0];
    let content_area = note_chunks[1];

    let is_notes_focused = focus(PanelId::WriterNotes);
    let border_color = if is_notes_focused {
        theme.accent
    } else {
        theme.surface
    };

    let list_inner_area = Block::default()
        .borders(Borders::RIGHT)
        .border_style(Style::default().fg(border_color))
        .inner(list_area);

    f.render_widget(
        Block::default()
            .borders(Borders::RIGHT)
            .border_style(Style::default().fg(border_color)),
        list_area,
    );

    let mut list_lines = Vec::new();
    let num_notes = snap.notes.len();
    if num_notes == 0 {
        list_lines.push(Line::from(vec![Span::styled(
            " No notes found in vault. Press e to create.",
            Style::default().fg(theme.dim),
        )]));
    } else {
        let max_idx = num_notes.saturating_sub(1);
        app.panel_states.writer_selected = app.panel_states.writer_selected.min(max_idx);
        let selected = app.panel_states.writer_selected;

        let visible_items = list_inner_area.height as usize;
        let mut scroll = app.panel_states.writer_scroll;
        if selected < scroll {
            scroll = selected;
        } else if selected >= scroll + visible_items && visible_items > 0 {
            scroll = selected.saturating_sub(visible_items - 1);
        }
        app.panel_states.writer_scroll = scroll;

        for (i, note) in snap
            .notes
            .iter()
            .enumerate()
            .skip(scroll)
            .take(visible_items)
        {
            if i == selected {
                list_lines.push(Line::from(vec![
                    Span::styled(" > ", Style::default().fg(theme.accent)),
                    Span::styled(&note.title, Style::default().fg(theme.text)),
                ]));
            } else {
                list_lines.push(Line::from(vec![
                    Span::raw("   "),
                    Span::styled(&note.title, Style::default().fg(theme.dim)),
                ]));
            }
        }
    }

    f.render_widget(Paragraph::new(list_lines), list_inner_area);

    let mut content_lines = Vec::new();
    if num_notes > 0 {
        let selected = app.panel_states.writer_selected;
        let note = &snap.notes[selected];
        content_lines.push(Line::from(vec![Span::styled(
            &note.title,
            Style::default().fg(theme.accent),
        )]));
        content_lines.push(Line::from(""));
        for line in note.content.lines() {
            let mut style = Style::default().fg(theme.text);
            if line.starts_with("# ")
                || line.starts_with("## ")
                || line.starts_with("### ")
                || line.starts_with("#### ")
            {
                style = style
                    .fg(theme.accent)
                    .add_modifier(ratatui::style::Modifier::BOLD);
            } else if line.starts_with("- ") || line.starts_with("* ") {
                style = style.fg(theme.yellow);
            } else if line.starts_with("> ") {
                style = style
                    .fg(theme.dim)
                    .add_modifier(ratatui::style::Modifier::ITALIC);
            } else if line.starts_with("```") {
                style = style.fg(theme.red);
            }
            content_lines.push(Line::from(Span::styled(line, style)));
        }
    }

    f.render_widget(
        Paragraph::new(content_lines).wrap(Wrap { trim: false }),
        content_area,
    );

    // Bottom Right: Files (Yazi)
    let files_inner = panel(
        f,
        files_area,
        "yazi (file manager)",
        theme,
        focus(PanelId::Files),
    );
    let files_focused = focus(PanelId::Files);
    crate::widgets::files::render(
        f,
        files_inner,
        theme,
        files_focused,
        &mut app.panel_states.files_selected,
        &mut app.panel_states.files_scroll,
    );
}
