# Vanta Monitor Architecture

This document explains the current production-oriented structure of the `monitor/` module inside the Vanta umbrella repo.

## Purpose

Vanta Monitor is a local-first system monitor with two surfaces:

1. Textual TUI (primary operator interface)
2. Flask web dashboard (secondary browser surface)

Both surfaces share the same collector and model layer so data formatting and behavior stay aligned.

## High-level flow

```text
SystemCollector
  -> SystemSnapshot models
    -> Presenter helpers
      -> Textual screens / Flask API
```

## Directory structure

```text
monitor/
├── pyproject.toml              # package metadata, scripts, pytest config
├── README.md                   # operator quickstart and feature docs
├── tests/                      # unit tests
└── src/monitor/
    ├── __main__.py             # CLI dispatcher (tui/web/both/help)
    ├── app.py                  # Textual app shell, nav bar, help binding
    ├── server.py               # Flask dashboard + /api/stats endpoint
    ├── dashboard.html          # inline browser dashboard UI
    ├── core/
    │   ├── models.py           # typed snapshot data structures
    │   ├── collectors.py       # psutil/pynvml system data collection
    │   ├── history.py          # bounded time-series buffer
    │   ├── process_service.py  # process list + kill/stop/resume actions
    │   ├── overview_presenter.py
    │   ├── process_presenter.py
    │   └── graph_presenter.py
    ├── screens/
    │   ├── overview.py         # dense summary screen
    │   ├── processes.py        # process operations screen
    │   ├── storage.py          # disks table
    │   ├── network.py          # live network stats
    │   ├── graphs.py           # large trend graph screen
    │   └── help_screen.py      # modal keybind overlay
    └── components/
        ├── process_table.py    # reusable process DataTable widget
        └── metric_card.py.deprecated
```

## Runtime surfaces

### 1. TUI

Entrypoints:
- `vmon`
- `vmon tui`
- `vtui`

Responsibilities:
- screen navigation
- keyboard-first monitoring
- process actions
- dense overview and graphs
- in-app help overlay via `?`

### 2. Web dashboard

Entrypoints:
- `vmon web`
- `vweb`

Responsibilities:
- expose `/api/stats`
- render browser dashboard from `dashboard.html`
- lightweight secondary monitoring surface

### 3. Dual mode

Entrypoints:
- `vmon both`
- `vboth`

Responsibilities:
- start Flask in a child process
- run Textual app in foreground

## Core design choices

### Shared collector

`SystemCollector` is the single source of truth for:
- CPU
- memory
- disks
- network throughput
- process count
- temperature
- optional NVIDIA GPU metrics

This avoids drift between TUI and web dashboard.

### Presenter layer

Formatting is pushed into presenter helpers instead of being embedded directly inside widgets.

Benefits:
- easier testing
- easier dense-layout iteration
- lower UI code complexity
- safer future refactors

Current presenters:
- `overview_presenter.py`
- `process_presenter.py`
- `graph_presenter.py`

### Bounded history buffers

Trend data uses `HistoryBuffer` to avoid unbounded memory growth.

Current usage:
- overview mini-sparklines
- graphs screen 60s / 600s trend windows

## Error-handling posture

Current protections:
- collector failures handled in overview/storage/network/graphs screens
- process actions wrapped in try/except
- disk permission errors tolerated
- temperature sensor failures tolerated
- GPU absence tolerated via optional pynvml fallback
- web API returns JSON errors instead of raw tracebacks

## Validation workflow

Local validation:

```bash
cd monitor
python -m py_compile src/monitor/*.py src/monitor/screens/*.py src/monitor/components/*.py src/monitor/core/*.py
pytest tests/ -v
```

CI validation:
- `.github/workflows/ci.yml`
- runs compile check + pytest on push/PR to `main`

## Known non-blocking gaps

- no Dockerfile
- no screen-level tests yet for storage/network/graphs
- web dashboard still depends on CDN-hosted GSAP
- only dark theme is implemented

## Safe extension points

If extending the app, prefer:
1. add or update data in `core/`
2. add formatting in presenter helpers
3. wire UI in screens/components last
4. cover behavior with tests before broad UI refactors
