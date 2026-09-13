import re
with open("src/app.rs", "r") as f: content = f.read()

bad = """        if self.mode == DashboardMode::Obsidian {
            match key.code {
                KeyCode::Up | KeyCode::Char('k') => {
                    ps.obsidian_selected = ps.obsidian_selected.saturating_sub(1);
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    ps.obsidian_selected = ps.obsidian_selected.saturating_add(1);
                }
                _ => {}
            }
        }"""

good = """        if self.mode == DashboardMode::Obsidian {
            match key {
                KeyCode::Up | KeyCode::Char('k') => {
                    self.panel_states.obsidian_selected = self.panel_states.obsidian_selected.saturating_sub(1);
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    self.panel_states.obsidian_selected = self.panel_states.obsidian_selected.saturating_add(1);
                }
                _ => {}
            }
        }"""

content = content.replace(bad, good)
with open("src/app.rs", "w") as f: f.write(content)

with open("src/app.rs", "r") as f: content = f.read()
content = content.replace("obsidian_scroll: 0,\n            obsidian_selected: 0,", "process_compact_cmd: false,\n            obsidian_scroll: 0,\n            obsidian_selected: 0,")
# wait, actually let's just make sure PanelStates is initialized properly.
