# Vanta Monitor

Vanta Monitor is a local-first system monitor with a Textual TUI as the primary surface and an optional Flask web dashboard for quick browser access.

What changed in this pass:
- dense overview screen with real process controls
- dedicated trend screen for CPU, memory, network throughput, and disk I/O
- production-style process workflow: live search, user/kernel filters, tree view, signal menu, detail modal
- responsive layout modes for short terminals
- named theme presets instead of a single dark/light pair

## Quick start

```bash
cd monitor
uv sync
vtui
```

Other entrypoints:
- `vmon tui` — launch the Textual TUI
- `vmon web` — launch the Flask dashboard on `http://localhost:5001`
- `vmon both` — start both surfaces

API readiness probes:
- `GET /api/health` — simple health check
- `GET /api/stats` — current machine snapshot
- `GET /api/processes` — filtered process list
- `GET /api/process/<pid>` — detailed process payload

## TUI screens

| Key | Screen | Purpose |
|-----|--------|---------|
| `1` | Overview | Main operator surface: CPU, memory, network, disks, process list |
| `2` | Graphs | Large trend panels for CPU, memory, network, and disk I/O |
| `3` | Files | Keyboard file manager / quick directory browser |

## Keybinds

### Global

| Key | Action |
|-----|--------|
| `1-3` | Switch screens |
| `?` | Help overlay |
| `T` | Toggle light/dark |
| `P` | Cycle theme preset |
| `r` | Refresh current screen |
| `q` | Quit |

### Overview screen

| Key | Action |
|-----|--------|
| `j` / `k` or arrows | Move process selection |
| `/` | Focus process search |
| `Esc` | Clear search |
| `c` / `C` | Cycle sort forward / backward |
| `u` | Toggle kernel processes |
| `U` | Toggle current-user filter |
| `F8` | Toggle flat / tree view |
| `d` | Open process detail modal |
| `K` | Open signal menu |
| Mouse click | Select process row |

## Theme presets

Current presets:
- `light`
- `dark`
- `monokai`
- `nord-light`

`T` flips between light/dark families fast. `P` cycles all presets.

## Config

Path: `config.json`

```json
{
  "ui": {"refresh_rate": 0.5, "theme": "light"},
  "process": {"show_kernel": false, "max_display": 15, "auto_refresh": true}
}
```

Notes:
- `ui.theme` may be any preset name listed above
- process search/filter state is session-local, not written back to config

## Architecture

```text
src/monitor/
├── __main__.py
├── app.py
├── server.py
├── core/
│   ├── collectors.py
│   ├── dashboard_config.py
│   ├── graph_presenter.py
│   ├── history.py
│   ├── models.py
│   ├── overview_presenter.py
│   ├── process_presenter.py
│   ├── process_service.py
│   └── theme.py
└── screens/
    ├── filemanager.py
    ├── graphs.py
    ├── help_screen.py
    └── overview.py
```

## Validation

```bash
cd monitor
uv run pytest -q
```

Current suite covers config loading, widgets/helpers, presenters, CLI dispatch, theme switching, overview screen interactions, graph screen rendering, and file manager navigation.
