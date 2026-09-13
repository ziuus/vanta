import re

# app.rs
with open("src/app.rs", "r") as f: content = f.read()

# Add Obsidian to Mode
content = re.sub(r'Settings,', 'Settings,\n    Obsidian,', content, count=1)

# Add keybind to switch to Obsidian (Mode::Obsidian) -> say, key '4'
content = re.sub(r'KeyCode::Char\(\'3\'\) => self\.mode = Mode::Settings,', r"KeyCode::Char('3') => self.mode = Mode::Settings,\n            KeyCode::Char('4') => self.mode = Mode::Obsidian,", content, count=1)

# Handle mode rendering
with open("src/app.rs", "w") as f: f.write(content)

# main.rs
with open("src/main.rs", "r") as f: content = f.read()
content = re.sub(r'Mode::Settings => crate::screens::settings::render\(f, app\),', r'Mode::Settings => crate::screens::settings::render(f, app),\n            crate::app::Mode::Obsidian => crate::screens::obsidian::render(f, app),', content, count=1)
with open("src/main.rs", "w") as f: f.write(content)

# screens/mod.rs
with open("src/screens/mod.rs", "r") as f: content = f.read()
content = re.sub(r'pub mod settings;', 'pub mod settings;\npub mod obsidian;', content, count=1)
with open("src/screens/mod.rs", "w") as f: f.write(content)

# config.rs
with open("src/config.rs", "r") as f: content = f.read()
content = re.sub(r'pub startup_mode: String,', 'pub startup_mode: String,\n    pub obsidian_vault: String,', content, count=1)
content = re.sub(r'startup_mode: "dashboard"\.to_string\(\),', r'startup_mode: "dashboard".to_string(),\n            obsidian_vault: "~/Documents/Obsidian".to_string(),', content, count=1)
with open("src/config.rs", "w") as f: f.write(content)

