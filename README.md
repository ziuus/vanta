# vanta

[🚀 Live Landing Page](website/index.html)

**Rust TUI system dashboard.** Single-pane, keyboard-driven, 6 dashboard modes.

![screenshot](docs/screenshot.png)

---

## Modes

| Key | Mode | What you see |
|-----|------|-------------|
| `1` | **Overview** | Full monitoring grid — CPU, memory, disk, network, GPU, clock, calendar, media player, music visualizer, system info, live process table |
| `2` | **Monitor** | Focused system metrics — CPU, memory, disk, network, GPU, system info |
| `3` | **Processes** | Full-width live process table with sort, search, tree view, collapse, detail view |
| `4` | **Media** | Large music visualizer + media player info + clock |
| `5` | **Aesthetic** | Clock, calendar, music visualizer, and matrix rain — four‑panel eye candy |
| `6` | **Settings** | Current theme, mode switching help, config file path, keyboard reference |

---

## Controls

| Key | Action |
|-----|--------|
| `1`–`6` | Switch dashboard mode |
| `T` | Cycle theme (dark → light → dracula → solarized‑light) |
| `Tab` / `Shift‑Tab` | Cycle panel focus |
| `↑` `↓` `←` `→` | Navigate focused panel (scroll processes, cycle calendar month) |
| `Esc` | Clear panel focus |
| `q` | Quit |

**Processes panel** (mode `3`):
| Key | Action |
|-----|--------|
| `s` | Cycle sort field |
| `/` | Enter search mode |
| `t` | Toggle tree view |
| `Space` | Collapse / expand tree node |
| `i` | Toggle detail view |
| `c` | Toggle compact command |
| `k` | Kill selected process |

**Media panel** (mode `4`):
| Key | Action |
|-----|--------|
| `Space` | Play / pause |
| `n` | Next track |
| `p` | Previous track |
| `+` / `-` | Volume up / down |

---

## Themes

Four built-in themes, cycled with `T`:

- `dark` — default, deep background
- `light` — light background
- `dracula` — purple‑accented dark
- `solarized-light` — warm light

The active theme is persisted to `~/.config/vanta/config.toml` and restored on next launch.

---

## Config

File: `~/.config/vanta/config.toml`

```toml
[ui]
refresh_rate = 0.5
theme = "dark"

[widgets]
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
```

Set any widget to `false` to hide it from the UI.

---

## Install

```bash
cargo install --git https://github.com/ziuus/vanta
```

Or build from source:

```bash
git clone https://github.com/ziuus/vanta
cd vanta
cargo run            # live monitoring
cargo run -- --demo  # demo mode (fake data)
```

---

## Stack

| Component | Tool |
|-----------|------|
| TUI framework | Ratatui 0.29 |
| Terminal backend | Crossterm |
| System info | sysinfo |
| Media control | playerctl (MPRIS) |
| Date/time | chrono |
| Rand | rand |

---

## License

MIT