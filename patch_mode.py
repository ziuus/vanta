import re

with open("src/mode.rs", "r") as f: content = f.read()

content = re.sub(r'Aesthetic,', 'Aesthetic,\n    Obsidian,', content, count=1)
content = re.sub(r'Self::Aesthetic => "aesthetic",', 'Self::Aesthetic => "aesthetic",\n            Self::Obsidian => "obsidian",', content, count=1)
content = re.sub(r'"aesthetic" => Self::Aesthetic,', '"aesthetic" => Self::Aesthetic,\n            "obsidian" => Self::Obsidian,', content, count=1)

with open("src/mode.rs", "w") as f: f.write(content)

with open("src/app.rs", "r") as f: content = f.read()
content = re.sub(r'DashboardMode::Aesthetic => vec!\[', 'DashboardMode::Obsidian => vec![],\n            DashboardMode::Aesthetic => vec![', content, count=1)
content = re.sub(r'KeyCode::Char\(\'3\'\) => self\.set_mode\(DashboardMode::Aesthetic\),', "KeyCode::Char('3') => self.set_mode(DashboardMode::Aesthetic),\n            KeyCode::Char('4') => self.set_mode(DashboardMode::Obsidian),", content, count=1)
content = re.sub(r'\(None, DashboardMode::Aesthetic\) => screens::aesthetic::render\(f, main, self\),', '(None, DashboardMode::Aesthetic) => screens::aesthetic::render(f, main, self),\n            (None, DashboardMode::Obsidian) => screens::obsidian::render(f, main, self),', content, count=1)
content = re.sub(r'DashboardMode::Aesthetic,', 'DashboardMode::Aesthetic,\n            DashboardMode::Obsidian,', content, count=1)
with open("src/app.rs", "w") as f: f.write(content)

with open("src/screens/mod.rs", "r") as f: content = f.read()
content = re.sub(r'pub mod settings;', 'pub mod settings;\npub mod obsidian;', content, count=1)
with open("src/screens/mod.rs", "w") as f: f.write(content)
