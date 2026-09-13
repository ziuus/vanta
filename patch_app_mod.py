import re

with open("src/screens/dashboard.rs", "r") as f:
    content = f.read()
content = re.sub(r'use crate::widgets::\{calendar, clock, cpu, disk, gauge, matrix, media, memory, meter, music_viz, network, status\};', 'use crate::widgets::{calendar, clock, cpu, disk, gauge, matrix, media, memory, meter, music_viz, network, status, weather};', content, count=1)
with open("src/screens/dashboard.rs", "w") as f:
    f.write(content)

with open("src/app.rs", "r") as f:
    content = f.read()
content = re.sub(r'&PanelId::Video => "video",', '&PanelId::Video => "video",\n            &PanelId::Weather => "weather",', content, count=1)
with open("src/app.rs", "w") as f:
    f.write(content)

with open("src/screens/mod.rs", "r") as f:
    content = f.read()
content = re.sub(r'PanelId::Video => crate::widgets::video::render\(f, inner, theme\),', 'PanelId::Video => crate::widgets::video::render(f, inner, theme),\n        PanelId::Weather => crate::widgets::weather::render(f, inner, theme),', content, count=1)
with open("src/screens/mod.rs", "w") as f:
    f.write(content)
