# Vanta Configuration & Customization

## Configuration

`~/.config/vanta/config.toml`. Every key is optional; delete the file to reset.

```toml
[ui]
refresh_rate = 0.5      # seconds between samples
fps = 30                # render rate (animations)
theme = "dark"
startup_mode = "dashboard"
clock_24h = true
visualizer = "bars"     # bars | mirror | wave | peaks

[widgets]               # all default to true
cpu = true
memory = true
disk = true
network = true
gpu = true
clock = true
calendar = true
music_viz = true
processes = true
media = true
matrix = true
video = true
```

## Dashboard Layout

You can completely customize the layout of the main Dashboard (Page 1) by defining a `[dashboard]` block in your `config.toml`. 

The `layout` parameter is an array of columns. Each column is an array of widget IDs arranged vertically.

```toml
[dashboard]
preset = "custom"
layout = [
    # Column 1 (Left)
    ["system", "gauges", "cpu", "storage"], 
    
    # Column 2 (Middle)
    ["clock", "media", "visualizer", "processes"], 
    
    # Column 3 (Right)
    ["status", "weather", "memory", "network", "calendar"]
]
```

### Swapping Widgets
You can remove widgets, rearrange them, or replace them entirely! If you install a community WASM extension (e.g., `coin`), or create a `[[custom_widgets]]` entry with `id = "pi_temp"`, you can simply drop that ID directly into the layout.

For example, to replace the semi-circle `gauges` with a spinning 3D `coin` extension:
```toml
layout = [
    ["system", "coin", "cpu", "storage"], 
    # ...
]
```

**Built-in Widget IDs:**
`system`, `gauges`, `cpu`, `storage`, `clock`, `media`, `visualizer`, `processes`, `status`, `weather`, `memory`, `network`, `calendar`.


## WASM Extensions (v0.10+)

Vanta is extensible through runtime WASM extensions. You can install completely new UI panels and capabilities created by the community *without* recompiling Vanta.

### 1. Discover Extensions
Search the community registry (hosted via the [vanta-integrations](https://github.com/ziuus/vanta-integrations) repo):
```bash
vanta search
```
```text
Vanta Extensions (API v0.9.1)

  security     Vanta Security
               Live CVE security feeds and threat monitoring.
               v0.1.0 (API v0.9.0) by Community
```

### 2. Install & Verify
Install an extension. Vanta will securely download the `.wasm` artifact, verify its **SHA-256 checksum**, check API compatibility, and place it in your local extensions folder:
```bash
vanta install security
```

> 🔒 **Security Notice**: Extensions are sandboxed by default. Currently, extensions are granted UI and Configuration access, but are denied Network, Filesystem, and Process execution capabilities.

### 3. Enable in Config
Enable the extension so Vanta loads it on startup:
```bash
vanta enable security
```
It will automatically append to the `enabled` array in your `~/.config/vanta/config.toml`. 

Start Vanta (`vanta`) and the new panel will appear in your dashboard! 

### Custom Pages (No Code Required)
You can still build custom views natively with flexible grid layouts by defining `[[pages]]` in your config:

```toml
[[pages]]
name = "DevOps"
layout = [
    ["system", "network"],
    ["processes", "clock"]
]
```

## Custom Widgets

Add your own data panels to the Dashboard without recompiling Vanta.  
Each `[[custom_widgets]]` entry in `config.toml` runs a command or reads a file in the background and displays the result.

### Quick examples

**Raspberry Pi voltage:**

```toml
[[custom_widgets]]
id = "pi_voltage"
title = "Pi Battery"
source = "command"
command = "vcgencmd measure_volts"
renderer = "value"
refresh = 2.0
```

**Raspberry Pi CPU temperature (millidegrees → degrees with min/max):**

```toml
[[custom_widgets]]
id = "pi_temp"
title = "CPU Temp"
source = "command"
command = "cat /sys/class/thermal/thermal_zone0/temp"
renderer = "gauge"
refresh = 2.0
min = 0
max = 100000
unit = "m°C"
```

**Linux battery percentage (gauge renderer):**

```toml
[[custom_widgets]]
id = "battery"
title = "Battery"
source = "file"
path = "/sys/class/power_supply/BAT0/capacity"
renderer = "gauge"
refresh = 1.0
min = 0
max = 100
```

**Any shell command as plain text:**

```toml
[[custom_widgets]]
id = "uptime_cmd"
title = "Uptime"
source = "command"
command = "uptime -p"
renderer = "text"
refresh = 10.0
```

**Docker running containers:**

```toml
[[custom_widgets]]
id = "docker_count"
title = "Containers"
source = "command"
command = "docker ps -q"
renderer = "text"
refresh = 5.0
```

### All options

| Key | Required | Default | Description |
|-----|----------|---------|-------------|
| `id` | ✓ | — | Unique identifier (used for focus/zoom). |
| `title` | ✓ | — | Panel title shown in the border. |
| `source` | — | `command` | `command` or `file`. |
| `command` | when `source = "command"` | — | Command to run (supports quoted args; no shell). |
| `path` | when `source = "file"` | — | File to read (e.g. `/sys/class/…`). |
| `renderer` | — | `value` | `value` · `text` · `gauge` · `bar` · `graph`. |
| `refresh` | — | `5.0` | Seconds between fetches (0.1 – 3600). |
| `unit` | — | none | Unit suffix appended to `value` displays (e.g. `"V"`). |
| `min` | — | `0` | Range minimum for `gauge` and `bar`. |
| `max` | — | `100` | Range maximum for `gauge` and `bar`. |
| `enabled` | — | `true` | Set `false` to hide without removing the entry. |

### Renderers

| Name | Shows |
|------|-------|
| `value` | Single value, centred and bold. Appends `unit` if set. |
| `text` | Raw text output, word-wrapped. |
| `gauge` | Horizontal bar `████░░ 82%` normalised to `min`/`max`. |
| `bar` | Horizontal bar without percentage label. |
| `graph` | Scrolling history graph using Vanta's block-character primitives. |

### How it works

- Each widget runs on its own background thread at its own `refresh` interval. The render loop (30 fps) only reads a cached result — it never blocks on I/O.
- Commands are executed directly (no `sh -c`). Arguments with spaces can be quoted: `command = 'grep -r "error" /var/log'`.
- Commands that hang are killed after 10 seconds. Crashed commands, missing files, and parse errors are shown as status messages (`Command failed`, `Unavailable`, `Invalid value`, `Timeout`, `Loading…`) — they never crash Vanta.
- Custom widgets appear as a horizontal row at the bottom of the Dashboard page. They participate in normal Tab focus and Enter zoom like any built-in panel.
- Set `VANTA_DEBUG=1` before running Vanta to see per-widget error messages on stderr.

