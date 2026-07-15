# vanta

Rust TUI system dashboard. Single‑pane, keyboard‑driven.

![screenshot](docs/screenshot.png)

---

**Monitoring** — CPU (per‑core usage, temp, freq), memory & swap, disk usage & I/O, network throughput with sparklines, GPU utilization & VRAM.

**Widgets** — Clock with battery, media player (MPRIS), calendar, music visualizer, system info.

**Processes** — Live process table with sort, search, tree view, dynamic row count, scroll.

**Controls** — Arrow/WASD panel focus · Tab/Shift‑Tab panel cycle · `Esc` focus reset · `/` search · `t` tree toggle · `Space` collapse · `q` quit · `T` theme toggle.

**Demo** — `cargo run -- --demo` for stable fake data (screenshot/README mode).

---

```bash
cargo run            # live monitoring
cargo run -- --demo  # screenshot/README mode
```

**Config:** `~/.config/vanta/config.toml` — widget toggles, refresh rate, theme.

**Stack:** Ratatui 0.29 + Crossterm · sysinfo · chrono · playerctl

**Screenshots:** See `docs/` for live monitoring and themed screenshots.
