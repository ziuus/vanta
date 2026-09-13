import re

# Fix tasks.rs
with open("src/monitors/tasks.rs", "r") as f: content = f.read()
config_dir = """pub fn get_todo_file() -> PathBuf {
    if let Ok(home) = std::env::var("HOME") {
        let mut path = PathBuf::from(home);
        path.push(".config");
        path.push("vanta");
        path.push("todo.md");
        path
    } else {
        PathBuf::from("todo.md")
    }
}"""
content = re.sub(r'pub fn get_todo_file\(\) -> PathBuf \{.*?^}$', config_dir, content, flags=re.MULTILINE|re.DOTALL)
with open("src/monitors/tasks.rs", "w") as f: f.write(content)

# Fix app.rs match
with open("src/app.rs", "r") as f: content = f.read()
content = re.sub(r'&PanelId::Weather => "weather",', '&PanelId::Weather => "weather",\n            &PanelId::Tasks => "tasks",', content, count=1)
# Make sure we didn't miss it if it was added incorrectly
content = re.sub(r'&PanelId::Weather => "weather",\n            &PanelId::Tasks => "tasks",\n            &PanelId::Tasks => "tasks",', '&PanelId::Weather => "weather",\n            &PanelId::Tasks => "tasks",', content)
with open("src/app.rs", "w") as f: f.write(content)

# Fix mod.rs match
with open("src/screens/mod.rs", "r") as f: content = f.read()
# Make sure PanelId::Tasks is handled in the render match
if 'PanelId::Tasks =>' not in content:
    content = re.sub(r'P::Weather => crate::widgets::weather::render\(f, inner, theme\),', 'P::Weather => crate::widgets::weather::render(f, inner, theme),\n        P::Tasks => crate::widgets::tasks::render(f, inner, theme),', content, count=1)
with open("src/screens/mod.rs", "w") as f: f.write(content)

