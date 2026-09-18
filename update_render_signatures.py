import sys
import glob

def patch(filename, modname):
    content = open(filename).read()
    old_sig = "pub fn render(f: &mut Frame, area: Rect, theme: &Theme) {"
    new_sig = "pub fn render(f: &mut Frame, area: Rect, theme: &Theme, is_detailed: bool) {"
    if old_sig in content:
        content = content.replace(old_sig, new_sig)
        open(filename, 'w').write(content)
        print(f"Updated {filename}")
    else:
        print(f"Could not find signature in {filename}")

for f in ["src/monitors/cpu.rs", "src/monitors/memory.rs", "src/monitors/disk.rs", "src/monitors/network.rs", "src/monitors/gpu.rs"]:
    patch(f, f.split('/')[-1].split('.')[0])

# Now update the callers in dashboard.rs and monitor.rs
dash_content = open("src/screens/dashboard.rs").read()
# Replace callers:
# cpu::render(f, inner, theme); -> cpu::render(f, inner, theme, false);
for mod in ["cpu", "memory", "disk", "network", "gpu"]:
    dash_content = dash_content.replace(f"{mod}::render(f, inner, theme);", f"{mod}::render(f, inner, theme, false);")
open("src/screens/dashboard.rs", "w").write(dash_content)
print("Updated dashboard.rs")

mon_content = open("src/screens/monitor.rs").read()
for mod in ["cpu", "memory", "disk", "network", "gpu"]:
    mon_content = mon_content.replace(f"{mod}::render(f, inner, theme);", f"{mod}::render(f, inner, theme, true);")
open("src/screens/monitor.rs", "w").write(mon_content)
print("Updated monitor.rs")
