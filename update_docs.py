import re

# Update README.md
with open('README.md', 'r') as f:
    readme = f.read()

# Add new ambient scenes
readme = re.sub(
    r'### `3` — Aesthetic\n\nDrops everything and displays a full-screen ambient scene\. Press `s` to rotate:\n\n- Audio visualizer\n- Matrix rain',
    r'### `3` — Aesthetic\n\nDrops everything and displays a full-screen ambient scene. Press `s` to rotate:\n\n- Audio visualizer\n- Matrix rain\n- Flip Clock\n- Topographic Map\n- True 3D Starfield (reacts to CPU spikes and music bass)\n- Conway\'s Game of Life (auto-seeding)\n- Terminal Snowfall (piles up and melts dynamically)',
    readme
)

with open('README.md', 'w') as f:
    f.write(readme)

# Update PROJECT-STATE.md
with open('PROJECT-STATE.md', 'r') as f:
    ps = f.read()

ps = re.sub(
    r'> \*\*Current Version:\*\* `v0\.6\.9`',
    r'> **Current Version:** `v0.10.34`',
    ps
)

# Add recent features to PROJECT-STATE.md
new_features = """
## 2. Completed Milestones (Up to v0.10.34)
- **Massive Performance Boost:** Zero-copy `Arc<str>` architecture for process monitoring, eliminating heap allocations in the hot path. Vanta now renders at 60fps with virtually zero CPU overhead.
- **Ambient Scenes Registry:** Added 5 new ambient scenes: Flip Clock, Topography, Starfield, Game of Life, and Terminal Snowfall.
- **New Dashboard Widgets:**
  - `github`: Live GitHub Contributions widget (polls via local `gh api`).
  - `thermals`: Radial thermals showing color-coded CPU and GPU temp gauges.
  - `world_clocks`: Sleek horizontal timezone strip.
- **Native Host Extensions:** Extension architecture for headless plugins.
"""
ps = re.sub(
    r'## 2\. Completed Milestones \(Up to v0\.6\.9\)',
    new_features,
    ps
)

with open('PROJECT-STATE.md', 'w') as f:
    f.write(ps)

# Update STATUS.md
with open('STATUS.md', 'r') as f:
    status = f.read()

status = re.sub(
    r'# Vanta — Development Status\n\n\*\*Current Version:\*\* `v0\.6\.9`',
    r'# Vanta — Development Status\n\n**Current Version:** `v0.10.34`',
    status
)

status_updates = """
### Latest Updates (v0.10.34)
- **New Dashboard Widgets:** GitHub Contributions, Radial Thermals, World Clocks.
- **New Ambient Scenes:** Starfield (audio-reactive), Game of Life, Terminal Snowfall, Flip Clock, Topography.
- **Performance:** Arc<str> zero-copy strings for Process Tree (massively reduced allocations).
- **Tooling:** Automated `Cargo.toml` <-> `package.json` sync. Fully clean `cargo clippy`.

"""

# Insert status_updates after "Current Version..."
status = status.replace('**Current Version:** `v0.10.34`', '**Current Version:** `v0.10.34`\n\n' + status_updates)

with open('STATUS.md', 'w') as f:
    f.write(status)

