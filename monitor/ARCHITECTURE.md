# Vanta Monitor Architecture

Vanta Monitor is a local-first monitoring package with one shared data layer and two operator surfaces:

1. Textual TUI — primary surface
2. Flask web dashboard — secondary surface

This pass intentionally narrows the architecture to what actually exists and is verified.

## High-level flow

```text
SystemCollector
  -> typed snapshot models
    -> presenter/helpers
      -> Textual screens / Flask API
```

## Runtime surfaces

### Textual TUI

The app shell owns:
- screen registration
- global navigation
- theme preset state
- help / refresh actions

Installed screens:
- `OverviewScreen` — main dashboard + process workflow
- `GraphsScreen` — dedicated long/short trend panels
- `FileManagerScreen` — keyboard file browser
- `HelpOverlay` — modal keybind sheet

### Web dashboard

The Flask app exposes:
- `/` for the browser dashboard
- `/api/health`
- `/api/stats`
- `/api/processes`
- `/api/process/<pid>`
- process action endpoints for kill / stop / resume

## Core modules

### `collectors.py`
Single source of truth for:
- CPU
- memory
- disks and disk I/O
- network throughput and totals
- process/thread counts
- temperature
- battery
- optional GPU metrics

### `history.py`
Bounded time-series buffers used by the overview status strip and graphs screen.

### `overview_presenter.py`
Dense formatting for overview cards, bars, and compact status helpers.

### `process_service.py`
Process listing and process actions:
- sort and reverse-sort helpers
- name/PID query filtering
- current-user filtering
- kernel inclusion toggle
- detail fetch with safe environment preview
- arbitrary signal dispatch through a fixed allowlist

### `theme.py`
Named palette presets plus helper functions:
- `get_palette(name)`
- `is_light_theme(name)`
- `next_theme_name(name)`

## Screen behavior

### Overview screen

Responsibilities:
- dense top-level machine state
- process selection by keyboard and mouse
- live search input
- tree/flat process modes
- process detail modal
- signal menu modal
- responsive compact/tiny layout handling

### Graphs screen

Responsibilities:
- CPU history
- memory history
- network throughput history
- disk I/O history
- short and long trend windows on one surface

### File manager screen

Responsibilities:
- browse directories
- inspect basic metadata
- preview folder/file summaries

## Validation workflow

```bash
cd monitor
uv run pytest -q
```

If you change core or screen code, keep the graphify graph current too:

```bash
graphify update .
```

## Current posture

What is production-ready enough now:
- live TUI navigation between the implemented screens
- process search/filter/sort/detail/signal workflow
- theme preset cycling
- responsive layout classes for short terminals
- tests for overview, graphs, file manager, presenters, config, and CLI dispatch

What is deliberately not claimed:
- a full btop clone
- six different monitoring screens
- storage/network/widgets screens that do not exist in this codebase
- remote monitoring or multi-host orchestration
