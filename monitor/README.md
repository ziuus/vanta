# Vanta Monitor

Vanta Monitor is the standalone Python monitor module inside Vanta.

It provides:
- Textual TUI
- Flask web dashboard
- shared config in `config.json`
- a packageable CLI entrypoint

## Run locally

```bash
cd ~/Projects/vanta/monitor
pip install -e .
vanta-monitor tui
vanta-monitor web
vanta-monitor both
```

## Short commands

```bash
vtui   # TUI
vweb   # Web dashboard
vboth  # TUI + web
vmon   # alias for TUI
```

## Why this exists

This keeps the monitor self-contained without splitting it into a separate product.
Vanta stays the umbrella project; `monitor/` stays independently runnable.
