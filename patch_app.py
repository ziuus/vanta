import re
with open("src/app.rs", "r") as f:
    content = f.read()

content = re.sub(r'    Video,\n    /// A user-defined', '    Video,\n    Weather,\n    /// A user-defined', content, count=1)
content = re.sub(r'\(\s*PanelId::Video,\s*w\.video\s*\),', '(PanelId::Video, w.video),\n                (PanelId::Weather, w.weather),', content, count=1)
content = re.sub(r'PanelId::Video => "donut",', 'PanelId::Video => "donut",\n            PanelId::Weather => "weather",', content, count=1)

with open("src/app.rs", "w") as f:
    f.write(content)
