# Vanta Monitor

Real-time system dashboard with a **Textual TUI** (primary) and an optional **Flask web dashboard**,
backed by a shared data layer.

## Quick start

```bash
cd monitor
pip install -e .
vtui               # TUI dashboard (default)
vmon web           # Web dashboard at :5001
vmon both          # TUI + web simultaneously
```

Shorter aliases: `vtui`, `vweb`, `vboth`. Also: `vmon tui`, `vmon web`, `vmon both`, `vmon help`.

## CLI usage

```bash
vmon [tui|web|both|help]
```

- `tui` — launch the Textual dashboard (default)
- `web` — launch the Flask dashboard on port 5001
- `both` — launch TUI + web dashboard together
- `help` — show CLI usage

## Screens (keys 1–5)

| Key | Screen | Content |
|-----|--------|---------|
| `1` | Dashboard | Dense system monitor core + paged extra widgets from config |
| `2` | Processes | Real-time process table with filter, kill/stop/resume, sort cycling |
| `3` | Storage | Disk mount usage table |
| `4` | Network | Live upload/download speed + total traffic |
| `5` | Graphs | Large detailed sparklines with dual-resolution history (60s + 600s) |

## Keybinds

### Global (all screens)

| Key | Action |
|-----|--------|
| `1-5` | Switch to screen |
| `?` | Open help overlay |
| `T` | Toggle light/dark theme |
| `q` | Quit |

### Dashboard (screen 1)

| Key | Action |
|-----|--------|
| `[` | Previous widget page |
| `]` | Next widget page |

### Processes (screen 2)

| Key | Action |
|-----|--------|
| `k` | Kill selected process |
| `s` | Stop (suspend) selected process |
| `r` | Resume selected process |
| `t` | Cycle sort column (cpu → mem → pid → threads → name) |
| `Ctrl+T` | Toggle sort direction (asc/desc) |
| `/` | Focus filter input |

## Features

### Dashboard panels
- **CPU** — per-core utilization in a 2-column grid, total %, frequency, load average, sparkline history
- **Memory** — total/used/free, percent bar with green/yellow/red color thresholds, swap
- **Network** — upload/download speed + cumulative totals
- **System** — GPU utilization + VRAM bars (if NVIDIA GPU detected via pynvml), temperature, uptime, process count
- **Disks** — per-mount usage bars
- **Disk IO** — aggregate read/write bps rates
- **Top processes** — CPU-sorted process preview

### Visual polish
- Color-coded utilization bars — green (<50%), yellow (<80%), red (≥80%) — on CPU, MEM, GPU, Disk panels
- Process table CPU/MEM columns color-graded by utilization
- Sort indicator with ▲▼ arrows in process status bar
- Process detail strip with colored values

### Theme
- **Light by default** — press `T` to toggle dark mode
- All screens and shell theme-aware simultaneously
- No individual screen stuck in wrong theme

### Responsive layout
- Automatically adapts to terminal width/height
- 3 modes: **full** (tall terminals), **compact** (medium), **tiny** (very small)
- Widget page size adjusts to available horizontal space

### Extras
- Config-driven widget dock (`config.json`) — enable/disable widgets, customize refresh rate
- Widget paging via `[` / `]`
- Command-backed widget caching (no jitter on repeated refresh)
- Help overlay with all keybinds

### Web dashboard
- Same data via Flask API at `/api/stats`
- Animated with GSAP
- Accessible at `http://localhost:5001`

## Config file

Path: `monitor/config.json`

```json
{
  "ui": {
    "refresh_rate": 0.5,
    "theme": "light"
  },
  "process": {
    "show_kernel": false,
    "max_display": 15
  },
  "widgets": {
    "dashboard": {"enabled": true},
    "clock": {"enabled": true},
    "calendar": {"enabled": true},
    "matrix": {"enabled": true},
    "music_viz": {"enabled": true},
    "pstree": {"enabled": true},
    "fastfetch": {"enabled": true},
    "custom_text": {"enabled": true},
    "process_manager": {"enabled": true},
    "system_stats": {"enabled": true}
  }
}
```

## Tests

```bash
pytest tests/ -v
```

70 tests covering: collectors, history, overview presenter, process service, process presenter,
graph presenter, CLI dispatch, dashboard overview/status/responsive/tiny modes, theme + toggle,
storage/network/graphs screen rendering, dashboard config + widget loading/caching/pagination.

## Architecture

```
src/monitor/
├── __main__.py                 # CLI entry points
├── app.py                      # Textual app shell (5 screens)
├── server.py                   # Flask web dashboard + API
├── core/
│   ├── models.py               # Data models (SystemSnapshot, etc.)
│   ├── collectors.py           # Shared SystemCollector (psutil + pynvml)
│   ├── history.py              # Bounded HistoryBuffer for sparklines
│   ├── process_service.py      # Process listing + kill/stop/resume
│   ├── overview_presenter.py   # Dense overview formatting + color bars
│   ├── process_presenter.py    # Process status/detail formatting
│   ├── graph_presenter.py      # Graph header/label/scale helpers
│   ├── dashboard_config.py     # Typed dashboard config + layout rules
│   └── dashboard_widgets.py    # Widget renderers + pagination/cache helpers
├── screens/
│   ├── overview.py             # Config-driven dashboard screen
│   ├── processes.py            # Process table with actions
│   ├── storage.py              # Disk usage table
│   ├── network.py              # Network stats cards
│   ├── graphs.py               # Large sparklines + dual-resolution history
│   └── help_screen.py          # Modal keybind overlay
└── components/
    └── process_table.py        # Reusable process DataTable + actions
```

See `ARCHITECTURE.md` for the full contributor-oriented structure.

## Why independent

Self-contained Python package inside the Vanta umbrella. Runs standalone (`pip install vanta-monitor`)
or as part of the larger `~/Projects/vanta` tmux dashboard suite.
