import re
with open("src/screens/dashboard.rs", "r") as f:
    content = f.read()

content = re.sub(r'use crate::widgets::\{calendar, clock, cpu, disk, gauge, matrix, media, memory, meter, music_viz, network, status\};', 'use crate::widgets::{calendar, clock, cpu, disk, gauge, matrix, media, memory, meter, music_viz, network, status, weather};', content, count=1)

rows_str = """        let rows = Layout::vertical([
            Constraint::Length(9),                                 // STATUS
            Constraint::Length(if cfg.weather { 9 } else { 0 }),   // WEATHER
            Constraint::Length(mem_h),                             // MEMORY (tall terminals)"""
content = re.sub(r'        let rows = Layout::vertical\(\[\n            Constraint::Length\(9\),                                 // STATUS\n            Constraint::Length\(mem_h\),                             // MEMORY \(tall terminals\)', rows_str, content, count=1)

render_str = """        if cfg.weather {
            let inner = panel(f, rows[1], "weather", theme, focus(PanelId::Weather));
            weather::render(f, inner, theme);
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
