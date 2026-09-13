import re

# monitors/mod.rs
with open("src/monitors/mod.rs", "r") as f: content = f.read()
content = re.sub(r'pub mod tasks;', 'pub mod tasks;\npub mod agenda;', content, count=1)
content = re.sub(r'tasks::start\(\);', 'tasks::start();\n    agenda::start();', content, count=1)
with open("src/monitors/mod.rs", "w") as f: f.write(content)

# widgets/mod.rs
with open("src/widgets/mod.rs", "r") as f: content = f.read()
content = re.sub(r'pub mod tasks;', 'pub mod tasks;\npub mod agenda;', content, count=1)
with open("src/widgets/mod.rs", "w") as f: f.write(content)

# config.rs
with open("src/config.rs", "r") as f: content = f.read()
content = re.sub(r'pub tasks: bool,', 'pub tasks: bool,\n    pub agenda: bool,', content, count=1)
content = re.sub(r'tasks: true,', 'tasks: true,\n            agenda: true,', content, count=1)
with open("src/config.rs", "w") as f: f.write(content)

# settings.rs
with open("src/screens/settings.rs", "r") as f: content = f.read()
content = re.sub(r'WidgetTasks,', 'WidgetTasks,\n    WidgetAgenda,', content, count=1)
content = re.sub(r'&SettingType::WidgetTasks => "Tasks",', '&SettingType::WidgetTasks => "Tasks",\n            &SettingType::WidgetAgenda => "Agenda",', content, count=1)
content = re.sub(r'&SettingType::WidgetTasks => app.config.ui.tasks,', '&SettingType::WidgetTasks => app.config.ui.tasks,\n            &SettingType::WidgetAgenda => app.config.ui.agenda,', content, count=1)
content = re.sub(r'&SettingType::WidgetTasks => app.config.ui.tasks = !app.config.ui.tasks,', '&SettingType::WidgetTasks => app.config.ui.tasks = !app.config.ui.tasks,\n            &SettingType::WidgetAgenda => app.config.ui.agenda = !app.config.ui.agenda,', content, count=1)
content = re.sub(r'SettingType::WidgetTasks,', 'SettingType::WidgetTasks,\n    SettingType::WidgetAgenda,', content, count=1)
with open("src/screens/settings.rs", "w") as f: f.write(content)

# app.rs
with open("src/app.rs", "r") as f: content = f.read()
content = re.sub(r'Tasks,', 'Tasks,\n    Agenda,', content, count=1)
content = re.sub(r'\(PanelId::Tasks, w.tasks\),', '(PanelId::Tasks, w.tasks),\n                (PanelId::Agenda, w.agenda),', content, count=1)
content = re.sub(r'&PanelId::Tasks => "tasks",', '&PanelId::Tasks => "tasks",\n            &PanelId::Agenda => "agenda",', content, count=1)
with open("src/app.rs", "w") as f: f.write(content)

# screens/mod.rs
with open("src/screens/mod.rs", "r") as f: content = f.read()
content = re.sub(r'P::Tasks => " TASKS ",', 'P::Tasks => " TASKS ",\n        P::Agenda => " AGENDA ",', content, count=1)
content = re.sub(r'P::Tasks => crate::widgets::tasks::render\(f, inner, theme\),', 'P::Tasks => crate::widgets::tasks::render(f, inner, theme),\n        P::Agenda => crate::widgets::agenda::render(f, inner, theme),', content, count=1)
with open("src/screens/mod.rs", "w") as f: f.write(content)

# dashboard.rs
with open("src/screens/dashboard.rs", "r") as f: content = f.read()
content = re.sub(r'Constraint::Length\(if cfg\.tasks \{ 10 \} else \{ 0 \}\),    // TASKS', 
                 r'Constraint::Length(if cfg.tasks { 10 } else { 0 }),    // TASKS\n            Constraint::Length(if cfg.agenda { 10 } else { 0 }),   // AGENDA', content, count=1)

render_agenda = """        if cfg.agenda {
            let snap = crate::monitors::agenda::snapshot();
            let count = snap.events.len();
            let agenda_rt = if count == 0 { " no events ".to_string() } else { format!(" {} upcoming ", count) };
            let inner = panel_full(f, rows[5], "agenda", Some(&agenda_rt), None, theme, focus(PanelId::Agenda));
            crate::widgets::agenda::render(f, inner, theme);
        }
    }"""
content = re.sub(r'        if cfg.tasks \{.*?crate::widgets::tasks::render\(f, inner, theme\);\n        \}\n    \}',
                 r'        if cfg.tasks {\n            let snap = crate::monitors::tasks::snapshot();\n            let open = snap.tasks.iter().filter(|t| !t.completed).count();\n            let tasks_rt = format!(" {} open ", open);\n            let inner = panel_full(f, rows[4], "tasks", Some(&tasks_rt), None, theme, focus(PanelId::Tasks));\n            crate::widgets::tasks::render(f, inner, theme);\n        }\n\n' + render_agenda, content, flags=re.DOTALL, count=1)
with open("src/screens/dashboard.rs", "w") as f: f.write(content)

