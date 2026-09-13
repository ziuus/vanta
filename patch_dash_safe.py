import re
with open("src/screens/dashboard.rs", "r") as f: content = f.read()

new_layout = """            Constraint::Min(6),                                // TOP PROCESSES
            Constraint::Length(if cfg.tasks { 10 } else { 0 }),    // TASKS
            Constraint::Length(if cfg.agenda { 8 } else { 0 }),   // AGENDA
            Constraint::Length(if cfg.news { 8 } else { 0 }),      // NEWS
        ]).spacing(1)"""
content = re.sub(r'            Constraint::Min\(6\),                                // TOP PROCESSES\n        \]\)\.spacing\(1\)', new_layout, content, count=1)

render_tasks = """        if cfg.tasks {
            let snap = crate::monitors::tasks::snapshot();
            let open = snap.tasks.iter().filter(|t| !t.completed).count();
            let tasks_rt = format!(" {} open ", open);
            let inner = panel_full(f, rows[4], "tasks", Some(&tasks_rt), None, theme, focus(PanelId::Tasks));
            crate::widgets::tasks::render(f, inner, theme);
        }

        if cfg.agenda {
            let snap = crate::monitors::agenda::snapshot();
            let count = snap.events.len();
            let agenda_rt = if count == 0 { " no events ".to_string() } else { format!(" {} upcoming ", count) };
            let inner = panel_full(f, rows[5], "agenda", Some(&agenda_rt), None, theme, focus(PanelId::Agenda));
            crate::widgets::agenda::render(f, inner, theme);
        }

        if cfg.news {
            let snap = crate::monitors::news::snapshot();
            let source = if snap.channel_title.is_empty() { " fetching ".to_string() } else { format!(" {} ", snap.channel_title) };
            let inner = panel_full(f, rows[6], "news", Some(&source), None, theme, focus(PanelId::News));
            crate::widgets::news::render(f, inner, theme);
        }
    }"""
content = re.sub(r'            crate::monitors::processes::render\(\n                f,\n                inner,\n                theme,\n                ps\.process_scroll_offset,\n                ps\.process_sort_field,\n                ps\.process_sort_asc,\n                &ps\.process_search,\n                ps\.process_search_active,\n                ps\.process_tree_mode,\n                &ps\.process_collapsed,\n                ps\.process_selected_pid,\n                ps\.process_compact_cmd,\n            \);\n        \}\n    \}', r'            crate::monitors::processes::render(\n                f,\n                inner,\n                theme,\n                ps.process_scroll_offset,\n                ps.process_sort_field,\n                ps.process_sort_asc,\n                &ps.process_search,\n                ps.process_search_active,\n                ps.process_tree_mode,\n                &ps.process_collapsed,\n                ps.process_selected_pid,\n                ps.process_compact_cmd,\n            );\n        }\n\n' + render_tasks, content, count=1)

with open("src/screens/dashboard.rs", "w") as f: f.write(content)
