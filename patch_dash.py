import re
with open("src/screens/dashboard.rs", "r") as f:
    content = f.read()

# Make sure we use panel_full
content = re.sub(r'use crate::screens::\{panel, too_small\};', 'use crate::screens::{panel, panel_full, too_small};', content, count=1)

# Modify CPU panel
content = re.sub(r'let inner = panel\(f, rows\[2\], "cpu", theme, focus\(PanelId::Cpu\)\);', 
                 r'let cpu_rt = format!(" {:.1}% ", sum.cpu_pct);\n            let inner = panel_full(f, rows[2], "cpu", Some(&cpu_rt), None, theme, focus(PanelId::Cpu));', content, count=1)

# Modify Memory panel
content = re.sub(r'let inner = panel\(f, rows\[2\], "memory", theme, focus\(PanelId::Memory\)\);', 
                 r'let mem_rt = format!(" {:.1}% ", sum.mem_pct);\n            let inner = panel_full(f, rows[2], "memory", Some(&mem_rt), None, theme, focus(PanelId::Memory));', content, count=1)

# Modify Network panel
content = re.sub(r'let inner = panel\(f, rows\[3\], "network", theme, focus\(PanelId::Network\)\);', 
                 r'let net_rt = format!(" ↓{:.0} ↑{:.0} kb/s ", sum.rx_kbps, sum.tx_kbps);\n            let inner = panel_full(f, rows[3], "network", Some(&net_rt), None, theme, focus(PanelId::Network));', content, count=1)

# Modify Weather panel
content = re.sub(r'let inner = panel\(f, rows\[1\], "weather", theme, focus\(PanelId::Weather\)\);', 
                 r'let weather_rt = if crate::monitors::weather::snapshot().ready { format!(" {} ", crate::monitors::weather::snapshot().location) } else { " offline ".to_string() };\n            let inner = panel_full(f, rows[1], "weather", Some(&weather_rt), None, theme, focus(PanelId::Weather));', content, count=1)

# Modify Calendar panel
content = re.sub(r'let inner = panel\(f, rows\[4\], "calendar", theme, focus\(PanelId::Calendar\)\);', 
                 r'let inner = panel_full(f, rows[4], "calendar", None, Some("← → month"), theme, focus(PanelId::Calendar));', content, count=1)

# Modify Top Processes panel
content = re.sub(r'let inner = panel\(\n            f,\n            rows\[3\],\n            "top processes",\n            theme,\n            focus\(PanelId::Processes\),\n        \);', 
                 r'let inner = panel_full(f, rows[3], "top processes", None, Some("↑ ↓ scroll • k kill"), theme, focus(PanelId::Processes));', content, count=1)

with open("src/screens/dashboard.rs", "w") as f:
    f.write(content)
