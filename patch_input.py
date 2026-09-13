import re
with open("src/app.rs", "r") as f: content = f.read()

nav_handling = """        if self.mode == DashboardMode::Obsidian {
            match key.code {
                KeyCode::Up | KeyCode::Char('k') => {
                    ps.obsidian_selected = ps.obsidian_selected.saturating_sub(1);
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    ps.obsidian_selected = ps.obsidian_selected.saturating_add(1);
                }
                _ => {}
            }
        }
        
        if self.mode == DashboardMode::Monitor && process_hotkey {"""

content = re.sub(r'        if self\.mode == DashboardMode::Monitor && process_hotkey \{', nav_handling, content, count=1)
with open("src/app.rs", "w") as f: f.write(content)

