import re

with open("src/screens/dashboard.rs", "r") as f:
    content = f.read()

rows_str = """        let rows = Layout::vertical([
            Constraint::Length(9),                                 // STATUS
            Constraint::Length(if cfg.weather { 9 } else { 0 }),   // WEATHER
            Constraint::Length(mem_h),                             // MEMORY (tall terminals)"""
content = re.sub(r'        let rows = Layout::vertical\(\[\n            Constraint::Length\(9\),                                 // STATUS\n            Constraint::Length\(mem_h\),                             // MEMORY \(tall terminals\)', rows_str, content, count=1)
render_str = """        if cfg.weather {
            let inner = panel(f, rows[1], "weather", theme, focus(PanelId::Weather));
            crate::widgets::weather::render(f, inner, theme);
        }

        if mem_h > 0 {
            let inner = panel(f, rows[2], "memory", theme, focus(PanelId::Memory));
            memory::render(f, inner, theme);
        }

        if cfg.network {
            let inner = panel(f, rows[3], "network", theme, focus(PanelId::Network));
            network::render(f, inner, theme);
        } else {
            let inner = panel(f, rows[3], "matrix", theme, false);
            matrix::render(f, inner, theme);
        }

        if cfg.calendar {
            let inner = panel(f, rows[4], "calendar", theme, focus(PanelId::Calendar));
            calendar::render(f, inner, theme, app.panel_states.calendar_month_offset);
        }"""
content = re.sub(r'        if mem_h > 0 \{\n            let inner = panel\(f, rows\[1\], "memory", theme, focus\(PanelId::Memory\)\);\n            memory::render\(f, inner, theme\);\n        \}.*?calendar::render\(f, inner, theme, app.panel_states.calendar_month_offset\);\n        \}', render_str, content, flags=re.DOTALL, count=1)
with open("src/screens/dashboard.rs", "w") as f:
    f.write(content)

with open("src/app.rs", "r") as f:
    content = f.read()
content = re.sub(r'Video,\n}', 'Video,\n    Weather,\n}', content, count=1)
content = re.sub(r'\(PanelId::Video,\n\s*w\.video\),', '(PanelId::Video, w.video),\n                (PanelId::Weather, w.weather),', content, count=1)
content = re.sub(r'&PanelId::Video => "video",', '&PanelId::Video => "video",\n            &PanelId::Weather => "weather",', content, count=1)
with open("src/app.rs", "w") as f:
    f.write(content)

with open("src/screens/mod.rs", "r") as f:
    content = f.read()
content = re.sub(r'P::Video => " VIDEO ",', 'P::Video => " VIDEO ",\n        P::Weather => " WEATHER ",', content, count=1)
content = re.sub(r'P::Video => crate::widgets::video::render\(f, inner, theme\),', 'P::Video => crate::widgets::video::render(f, inner, theme),\n        P::Weather => crate::widgets::weather::render(f, inner, theme),', content, count=1)
with open("src/screens/mod.rs", "w") as f:
    f.write(content)
