import re
with open("src/app.rs", "r") as f: content = f.read()
content = re.sub(r'PanelId::Custom\(_\) => "custom",', 'PanelId::Custom(_) => "custom",\n            &PanelId::Tasks => "tasks",', content, count=1)
with open("src/app.rs", "w") as f: f.write(content)
