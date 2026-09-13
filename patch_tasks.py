import re

# monitors/mod.rs
with open("src/monitors/mod.rs", "r") as f: content = f.read()
content = re.sub(r'pub mod weather;', 'pub mod weather;\npub mod tasks;', content, count=1)
content = re.sub(r'weather::start\(\);', 'weather::start();\n    tasks::start();', content, count=1)
with open("src/monitors/mod.rs", "w") as f: f.write(content)

# widgets/mod.rs
with open("src/widgets/mod.rs", "r") as f: content = f.read()
content = re.sub(r'pub mod weather;', 'pub mod weather;\npub mod tasks;', content, count=1)
with open("src/widgets/mod.rs", "w") as f: f.write(content)

# config.rs
with open("src/config.rs", "r") as f: content = f.read()
content = re.sub(r'pub weather: bool,', 'pub weather: bool,\n    pub tasks: bool,', content, count=1)
content = re.sub(r'weather: true,', 'weather: true,\n            tasks: true,', content, count=1)
with open("src/config.rs", "w") as f: f.write(content)

# settings.rs
with open("src/screens/settings.rs", "r") as f: content = f.read()
content = re.sub(r'WidgetWeather,', 'WidgetWeather,\n    WidgetTasks,', content, count=1)
content = re.sub(r'&SettingType::WidgetWeather => "Weather",', '&SettingType::WidgetWeather => "Weather",\n            &SettingType::WidgetTasks => "Tasks",', content, count=1)
content = re.sub(r'&SettingType::WidgetWeather => app\.config\.ui\.weather,', '&SettingType::WidgetWeather => app.config.ui.weather,\n            &SettingType::WidgetTasks => app.config.ui.tasks,', content, count=1)
content = re.sub(r'&SettingType::WidgetWeather => app\.config\.ui\.weather = !app\.config\.ui\.weather,', '&SettingType::WidgetWeather => app.config.ui.weather = !app.config.ui.weather,\n            &SettingType::WidgetTasks => app.config.ui.tasks = !app.config.ui.tasks,', content, count=1)
content = re.sub(r'SettingType::WidgetWeather,', 'SettingType::WidgetWeather,\n    SettingType::WidgetTasks,', content, count=1)
with open("src/screens/settings.rs", "w") as f: f.write(content)

# app.rs
with open("src/app.rs", "r") as f: content = f.read()
content = re.sub(r'Weather,', 'Weather,\n    Tasks,', content, count=1)
content = re.sub(r'\(PanelId::Weather, w\.weather\),', '(PanelId::Weather, w.weather),\n                (PanelId::Tasks, w.tasks),', content, count=1)
content = re.sub(r'&PanelId::Weather => "weather",', '&PanelId::Weather => "weather",\n            &PanelId::Tasks => "tasks",', content, count=1)
with open("src/app.rs", "w") as f: f.write(content)

# screens/mod.rs
with open("src/screens/mod.rs", "r") as f: content = f.read()
content = re.sub(r'P::Weather => " WEATHER ",', 'P::Weather => " WEATHER ",\n        P::Tasks => " TASKS ",', content, count=1)
content = re.sub(r'P::Weather => crate::widgets::weather::render\(f, inner, theme\),', 'P::Weather => crate::widgets::weather::render(f, inner, theme),\n        P::Tasks => crate::widgets::tasks::render(f, inner, theme),', content, count=1)
with open("src/screens/mod.rs", "w") as f: f.write(content)

# dashboard.rs
with open("src/screens/dashboard.rs", "r") as f: content = f.read()
# Let's insert tasks in the middle column right below the top processes
content = re.sub(r'Constraint::Min\(6\),                                // TOP PROCESSES\n        \]\)\.spacing\(1\);', 'Constraint::Min(6),                                // TOP PROCESSES\n            Constraint::Length(if cfg.tasks { 10 } else { 0 }),    // TASKS\n        ]).spacing(1);', content, count=1)

render_tasks = """        if cfg.tasks {
            let snap = crate::monitors::tasks::snapshot();
            let open = snap.tasks.iter().filter(|t| !t.completed).count();
            let tasks_rt = format!(" {} open ", open);
            let inner = panel_full(f, rows[4], "tasks", Some(&tasks_rt), None, theme, focus(PanelId::Tasks));
            crate::widgets::tasks::render(f, inner, theme);
        }
    }"""
content = re.sub(r'            crate::monitors::processes::render\(\n                f,\n                inner,\n                theme,\n                ps\.process_scroll_offset,\n                ps\.process_sort_field,\n                ps\.process_sort_asc,\n                &ps\.process_search,\n                ps\.process_search_active,\n                ps\.process_tree_mode,\n                &ps\.process_collapsed,\n                ps\.process_selected_pid,\n                ps\.process_compact_cmd,\n            \);\n        \}\n    \}', r'            crate::monitors::processes::render(\n                f,\n                inner,\n                theme,\n                ps.process_scroll_offset,\n                ps.process_sort_field,\n                ps.process_sort_asc,\n                &ps.process_search,\n                ps.process_search_active,\n                ps.process_tree_mode,\n                &ps.process_collapsed,\n                ps.process_selected_pid,\n                ps.process_compact_cmd,\n            );\n        }\n\n' + render_tasks, content, count=1)

with open("src/screens/dashboard.rs", "w") as f: f.write(content)

