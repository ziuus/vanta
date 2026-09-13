import re
with open("src/screens/dashboard.rs", "r") as f: content = f.read()

replacement = """        if cfg.processes {
            let inner = panel(f, rows[3], "processes", theme, focus(PanelId::Processes));
            let ps = &app.process_state;
            crate::monitors::processes::render(
                f,
                inner,
                theme,
                ps.process_scroll_offset,
                ps.process_sort_field,
                ps.process_sort_asc,
                &ps.process_search,
                ps.process_search_active,
                ps.process_tree_mode,
                &ps.process_collapsed,
                ps.process_selected_pid,
                ps.process_compact_cmd,
            );
        }
        
        if cfg.tasks {
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
    }
}
"""

content = re.sub(r'        if cfg.processes \{.*\}', replacement, content, flags=re.DOTALL)
with open("src/screens/dashboard.rs", "w") as f: f.write(content)
