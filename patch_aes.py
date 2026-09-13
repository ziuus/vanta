import re
with open("src/screens/aesthetic.rs", "r") as f:
    content = f.read()

content = re.sub(r'use crate::screens::\{panel, too_small\};', 'use crate::screens::{panel, panel_full, too_small};', content, count=1)

content = re.sub(r'let inner = panel\(f, top\[1\], "calendar", theme, focus\(PanelId::Calendar\)\);', 
                 r'let inner = panel_full(f, top[1], "calendar", None, Some("← → month"), theme, focus(PanelId::Calendar));', content, count=1)

with open("src/screens/aesthetic.rs", "w") as f:
    f.write(content)
