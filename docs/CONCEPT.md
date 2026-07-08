# Vanta — Aesthetic Rust TUI System Dashboard

> **Status:** Functional ✅ — 3,300+ lines of Rust, Ratatui 0.29 + Crossterm, actively developed

Vanta is a terminal-native system dashboard that combines btop-grade monitoring with flex/showoff terminal widgets, all in one unified single-pane TUI. No tabs, no windows — everything visible at once.

---

## Core Philosophy

**The single-pane dashboard.** Unlike btop/bottom (which use tabbed or panel-based views), Vanta renders everything in one 3-column grid. You see CPU, memory, network, disk, GPU, processes, clock, calendar, music visualizer, media controls, system info, and battery all at the same time. Nothing is hidden behind a tab switch.

**Built for Linux aesthetics.** Dark charcoal theme by default, per-core CPU bars, sparklines for disk/network history, green/yellow/red color thresholds. It feels at home in tmux with a powerline prompt and a TWM (DriftWM, Hyprland, etc.).

**Lightweight.** No web runtime, no GPU compositor, no heavy JS framework. Pure Ratatui + Crossterm, ~3.3k lines of Rust, compiles in seconds. Runs on any terminal emulator.

**Keyboard-first.** Everything through keyboard — Tab to cycle focus, arrow keys, search with `/`, kill with `k`, sort with `s`, tree mode with `t`. No mouse required.

---

## Architecture

### Stack

| Layer | Choice |
|-------|--------|
| TUI Framework | [Ratatui](https://ratatui.rs/) 0.29 |
| Backend | Crossterm 0.28 (Linux, macOS, Windows) |
| System Monitoring | [sysinfo](https://crates.io/crates/sysinfo) 0.33 (CPU, memory, disk, network, processes) |
| GPU | NVIDIA via nvidia-smi subprocess + sysinfo fallback |
| Clock | chrono + sysinfo uptime |
| Audio Viz | Pure math (no cpal or FFT) — generates animated bars from tick count |
| Media | playerctl (MPRIS D-Bus) via subprocess |
| Battery | sysfs (`/sys/class/power_supply/BAT*`) |
| System Info | /proc, /etc, uname, environment variables |
| Config | TOML via serde |
| Processing | rand crate for visualizer variation |

### Data Flow

```
[Every tick @ refresh_rate]
       │
       ▼
main.rs: read_input() ───→ handle_panel_nav()
       │                       │
       ▼                       ▼
  app.rs: summarize()      config/widget states
  ─────────────────
  Summary bar line:
  "vanta  dark  ·  up 3h  ·  CPU 11%  ·  MEM 76%  ·  GPU 0%  ·  ↓512KB  ↑120KB  ·  🔋100%  [T]heme  [q]uit"
       │
       ▼
  overview.rs: 40/60 layout
       │
       ├── Top 40%: 3-column metric/widget grid
       │   ├── Column 1: cpu::render(), memory::render(), network::render()
       │   ├── Column 2: clock::render(), [media::render()], calendar::render(), disk::render()
       │   └── Column 3: gpu::render(), music_viz::render(), system_info::render()
       │
       └── Bottom 60%: Full-width processes::render()
```

Each `render()` function receives a `Rect` (its allocated rectangle) and a `&Theme`, and draws directly via Ratatui widgets (Gauge, Paragraph, Sparkline, List, etc.). System data is fetched fresh on each render call — no caching layer yet (fast enough with sysinfo).

### Module Structure

```
src/
├── main.rs              # Event loop, input handling, App lifecycle
├── app.rs               # App struct, PanelId enum, PanelStates, focus/key logic, summary bar
├── config.rs            # TOML config deserialization (WidgetConfig, UiConfig)
├── monitors/
│   ├── mod.rs
│   ├── cpu.rs           # Per-core 2-across gauges + inline temp/freq + load avg
│   ├── memory.rs        # RAM + swap gauges, cached/buffers, sparkline
│   ├── disk.rs          # Mounted filesystem gauges + IO rates + sparklines
│   ├── network.rs       # DL/UL rates, short & long term sparklines
│   ├── gpu.rs           # NVIDIA: temp, util, VRAM, ECC, clock, fan
│   ├── processes.rs     # Flat or tree view, sort/search/kill, scrollable
│   └── system_info.rs   # OS, WM, kernel, uptime, shell, terminal, CPU model, GPU model
├── screens/
│   ├── mod.rs
│   ├── overview.rs      # 40/60 layout, panel rendering dispatch
│   └── widgets.rs       # panel() helper — bordered blocks with focus state
└── widgets/
    ├── mod.rs
    ├── clock.rs          # Big time, date, uptime, moon phase, battery (multi-BAT)
    ├── calendar.rs       # Month view with nav (◀▶), month offset
    ├── media.rs          # MPRIS via playerctl: play state, progress bar, controls
    └── music_viz.rs      # Animated audio bars (pseudo-FFT from tick + rand)
```

---

## Current Features (All Implemented)

### Monitoring (btop-grade)

- **CPU** — Compact header line showing total CPU%, load average, and active core count. Temperature shown when available. 2 cores per row with inline Gauge widgets showing percentage, and each core has temperature and frequency appended inline where available (e.g. `c0  12.3% · 52°C · 1800MHz`). No redundant total gauge — the per-core gauges and header line are enough.
- **Memory** — RAM gauge (used/total) with percentage label, swap gauge + percentage, cached+buffers+SReclaimable breakdown, history sparkline showing recent usage trend.
- **Disk** — Per-mountpoint gauges with used/total label and percentage. IO read/write rates with short and long-term sparklines per device.
- **Network** — Real-time download/upload rates displayed inline. Short-term (10 data points) and long-term (50 data points) sparklines per direction.
- **GPU (NVIDIA)** — Temperature, utilization %, VRAM used/total with gauge, ECC error count (if any), clock speed, fan speed. HWmon fallback when nvidia-smi is unavailable.
- **Processes** — Full-width panel at the bottom. Sortable by CPU/mem/PID/name with sort direction indicator (▴▾). Searchable with `/` live filter. Killable with `k` (SIGTERM). Tree view toggleable with `t`, collapsible parent rows with ←/→. Status bar shows `[T]` or `[F]` for tree/flat mode name, and for the sort field.

### System Info Widget

- **Purpose:** Screenshot-ready system identity card for sharing Vanta screenshots. Shows everything needed to identify the setup at a glance.
- **Line 1:** OS short name · hostname · kernel version
- **Line 2:** CPU model string · GPU model string
- **Line 3:** Desktop environment · shell · terminal emulator
- All values are detected from /proc, environment variables, uname, and nvidia-smi. Graceful fallbacks when values are missing.

### Flex Widgets

- **Clock** — 24h time (HH:MM:SS), date (Day, Mon DD YYYY), uptime, moon phase (8 phases with Unicode characters). Battery section below: per-battery gauge (multi-BAT support) with visual bar, capacity %, status (Charging/Discharging/Not charging), time remaining estimate in hours:minutes.
- **Calendar** — Month grid view with today highlighted (yellow). Navigate months with ◀▶ arrows, Home resets to current month.
- **Music Visualizer** — Animated audio bars in terminal green. 12 bars, 6 rows of █ blocks, randomized per tick with smooth falloff. Pure math, no audio capture needed.
- **Media Controls (MPRIS)** — When a media player is running (Spotify, Firefox, VLC, MPV), shows play/pause icon, artist — title, progress bar with elapsed/total time, player name. Keys: Space (play-pause), N (next), P (previous). Graceful "no media player active" message when no player is running. Uses `playerctl` subprocess with 0-warning error handling.

### Process Manager

- Flat list mode (default) — sortable columns (CPU▴▾, MEM▴▾, PID▴▾, NAME▴▾ indicators), scrollable
- Tree mode (`t`) — parent/child hierarchy with `│  ├───` indentation using custom tree building (no Ratatui Tree widget)
- Collapse (`←`) / expand (`→`) individual process subtrees in tree mode
- Search (`/`) filters by name, works in both flat and tree modes
- Kill (`k`) sends SIGTERM to the process at the current scroll position
- Status bar shows `Sort:[field] [Tree]` or `Sort:[field]` or `Sort:[field] | Search:[/term]` when focused

### Summary Bar

- Single-line top bar showing:
  - OS short name (e.g. "Arch" from /etc/os-release)
  - Theme symbol (🌙 dark / ☀ light)
  - Uptime short (e.g. "3h 12m")
  - CPU total %
  - MEM total %
  - GPU total %
  - Network down/up rates
  - Battery % when present
  - `[T]heme` and `[q]uit` indicators
- Example: `vanta  🌙  ·  up 3h 12m  ·  CPU 11%  ·  MEM 76%  ·  GPU 0%  ·  ↓512KB  ↑120KB  ·  🔋100%     [T]heme  [q]uit`

### UX

- **40/60 layout** — Top 40% of screen = 3-column metric/widget grid, bottom 60% = full-width processes panel. This was a direct response to feedback that the old equal-3-row layout wasted vertical space on processes.
- **Tab cycling** — Tab/Shift+Tab to move focus between panels (11 panels total: CPU → Clock → Memory → GPU → Calendar → Visualizer → Network → Disk → Processes → Media → System → wrap)
- **Focus borders** — thick/thin border with focus color for the focused panel
- **Arrow navigation** — Up/Down for scrollable panels (processes), Left/Right for calendar month nav, expand/collapse process parent
- **Status bar** — Focus panel name, process sort/tree/search hints
- **Config-driven** — TOML config to enable/disable any widget
- **Dark/light toggle** — [T]heme key switches between dark and light modes
- **Compact CPU** — 2 core gauges per row with inline temp and freq, saving vertical space compared to traditional 1-per-row layouts

---

## Key Bindings

| Key | Action |
|-----|--------|
| `Tab` | Cycle focus to next panel |
| `Shift+Tab` | Cycle focus to previous panel |
| `Esc` | Unfocus (no panel focused) |
| `T` | Toggle dark/light theme |
| `Q`, `Ctrl+C` | Quit |
| *(Panel-specific)* | |
| **Processes focused:** | |
| `↑`/`↓` | Scroll process list |
| `PgUp`/`PgDn` | Scroll 6 lines at a time |
| `s` | Cycle sort field (CPU/mem/PID/name) |
| `/` | Enter search mode, type name |
| `Esc` (in search) | Exit search mode |
| `k` | Kill selected process (SIGTERM) |
| `t` | Toggle flat/tree view |
| `←` | Collapse parent (tree mode) |
| `→` | Expand parent (tree mode) |
| **Calendar focused:** | |
| `←`/`→` | Previous/next month |
| `Home` | Reset to current month |
| **Media focused:** | |
| `Space` | Play/pause |
| `n` | Next track |
| `p` | Previous track |

---

## Configuration

TOML file at `~/.config/vanta/config.toml` or `$XDG_CONFIG_HOME/vanta/config.toml`:

```toml
[ui]
refresh_rate = 0.5        # Seconds between refreshes
theme = "dark"             # "dark" or "light"

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

Any widget set to `false` is hidden from the layout. System Info is always on (it's small, always useful).

---

## Layout

```
┌─────────────────────────────────────────────────────────────────────────────────────────────┐
│ vanta  🌙  ·  up 3h 12m  ·  CPU 11%  ·  MEM 76%  ·  GPU 0%  ·  ↓512KB  ↑120KB  ·  🔋100%    │  ← Summary bar
├───────────────────────────┬───────────────────────────┬─────────────────────────────────────┤
│  ▂▁▃  CPU                  │  ⏱  Clock                 │  🎮  GPU                           │  ← Top 40%
│  CPU 11%  load 1.4 1.2 0.9│  13:45:22                  │  ░▓▓▓▓░░  45°C  30%                │
│  c0 ██████████  12.3%      │  Sun, Jul 05 2026          │  VRAM ░░░███  2.4/8.0 GB           │
│      · 52°C · 1800MHz      │  Uptime 3h 12m             │                                    │
│  c1 ██████████  11.8%      │  🌒 Waning Crescent        │  ─── ─── ─── ─── ─── ─── ───      │
│      · 48°C · 1200MHz      │  🔋 ████████████  100%     │  🎵  Visualizer                    │
│  ...                        │    Not charging           │  ██                               │
│                             │                           │  ██ ██                             │
│  🧠  Memory                  │  ─── ─── ─── ─── ─── ───  │  ██ ██ ██                           │
│  ████████████████▁▁  78%    │  🎵  Media                 │  ██ ██ ██ ██                       │
│  12.4/15.6 GB               │  ▶ Playing                │  ██████████████████████            │
│  Swap ██░░░░  2.1/8.0 GB    │  Artist — Song Title      │                                    │
│  ▁▃▅▇█▇▅▃                   │  █████████████░░░  1:23   │  ─── ─── ─── ─── ─── ─── ───      │
│                             │                           │  🖥  System                        │
│  🌐  Network                  │  📅  Calendar              │  Arch · desktop                   │
│  ████████████▁▁  12.3MB/s   │      July 2026             │  Linux 6.7.0-arch1-1              │
│  ████████░░░░░░   2.1MB/s   │  Mo Tu We Th Fr Sa Su     │  i5-8265U · MX250                 │
│  ─▁▃▅▇▆▄▂▁▁▃▅▇▆▄▂          │           1  2  3  4  5    │  COSMIC · fish · kitty            │
│  ─▁▃▅▆▄▂▁▁▃▅▆▄▂           │   6  7  8  9 10 11 12     │                                    │
│                             │  13 14 15 16 17 18 19     │                                    │
│  💾  Disk                    │  20 21 22 23 24 25◀26     │                                    │
│  /      ████████░░  78%     │  27 28 29 30 31           │                                    │
│          R: 45K/s W: 12K/s  │                           │                                    │
│  /home  ████░░░░░░  42%     │                           │                                    │
│          R: 2K/s  W: 18K/s │                           │                                    │
├───────────────────────────┴───────────────────────────┴─────────────────────────────────────┤
│  ⚙  Processes [T]  CPU▾ MEM▴ PID NAME                                                      │  ← Bottom 60%
│  1034  init          0.1%   2.1M                                                           │  (full width)
│  2039  systemd-...   0.3%   8.4M                                                           │
│  ├── 3042  pipewire  0.2%   5.1M                                                           │
│  ├── 3043  wireplumb 0.1%   3.2M                                                           │
│  │   └── 4219  dbus...  0.0% 1.1M                                                          │
│  └── 4218  login...  0.0%   0.8M                                                           │
│  4421  firefox     4.2%  412.5M                                                            │
│  1239  Vanta       0.5%   7.2M                                                             │
│                                                                                             │
└─────────────────────────────────────────────────────────────────────────────────────────────┘
│ vanta v0.1.0  Focus: Processes  Sort:MEM [Tree]                                             │  ← Status bar
```

---

## Implementation Details

### Why the 40/60 layout?

First ChatGPT review feedback highlighted that the old 3-equal-row layout wasted vertical space: processes only got 1/9 of screen height in a 3x3 grid, despite being the most information-dense panel. The 40/60 split gives processes the full panel width and 60% of the screen height, while the top section remains dense and scroll-free for the monitoring widgets. This also matches btop's approach of giving processes prominence.

### Compact CPU design

Instead of a total gauge plus one core per row (btop default), Vanta's CPU panel shows 2 cores per row using narrower Gauge widgets inline with temperature and frequency data. The header line shows total CPU%, load average, and core count. This saves vertical space without losing information density.

### Summary bar implementation

Data is collected in `app.rs` via `summarize()` which reads battery sysfs, averages CPU/memory/GPU stats, and formats a single-line string. Rendered by the `render_title()` method using `Paragraph` with styled `Span` chunks for color coding. The summary bar is not a separate panel — it's rendered in the terminal title bar area.

### Why no cpal/mic audio capture?

The music visualizer uses pure math (random seed + tick-driven sine waves with smooth falloff) instead of actual audio capture. This means:
- Works in any terminal without audio permissions or PulseAudio/ALSA setup
- Zero CPU overhead
- Always looks good regardless of what's playing
- The trade-off: it doesn't actually respond to audio — it's aesthetic, not analytical

If you need a real FFT visualizer, cpal + rustfft could replace it, but the current approach is intentional for the "flex widget" vibe.

### Process tree implementation

The tree view builds a parent-child tree from `/proc/{pid}/stat` (field 4 = ppid), flattens it for display with depth-based indentation (`│  ├───`), and maintains a `HashSet<u32>` of collapsed PIDs. Sorting affects only top-level siblings. Tree mode is toggleable independently from flat sort/search state. Custom implementation — does not use Ratatui's experimental Tree widget.

### Battery reading

Scans `/sys/class/power_supply/BAT*`, reads capacity (%), status (Charging/Discharging/Not charging), power_now (µW), energy_now (µWh), energy_full (µWh). Time remaining calculated as `(energy_now / power_now) * 60` for minutes (Discharging) or time to full (Charging). Multi-battery support — each battery gets its own gauge + label line. "Not charging" status on AC is handled explicitly.

### Media (MPRIS) integration

Shells out to `playerctl` on each render with metadata format `{{playerName}}|{{artist}}|{{title}}|{{status}}|{{position}}|{{mpris:length}}`. Position/length are microsecond-precision u64 parsed from stdout. No active player → graceful "no media player active" empty state. 0-warning error handling path — initial `playerctl status` check avoids spamming metadata commands when no player is running.

### System Info detection

Reads from multiple sources with graceful fallbacks:
- OS name: /etc/os-release (NAME field)
- Hostname: /etc/hostname
- Kernel: uname -r from /proc/sys/kernel/ostype + release
- Uptime: /proc/uptime
- Shell: $SHELL env var (basename extracted)
- Terminal: $TERM_PROGRAM or $TERMINAL
- Desktop: $XDG_CURRENT_DESKTOP
- CPU model: /proc/cpuinfo (model name)
- GPU model: nvidia-smi --query-gpu=name

---

## What's Not Yet Done (Compared to README Vision)

| Feature | Status | Notes |
|---------|--------|-------|
| CPU temps/freq | ✅ Done | Inline in per-core gauges |
| Memory + swap + history | ✅ Done | Cache/buffers breakdown, sparkline |
| Network rates + sparklines | ✅ Done | Short + long term |
| Disk usage + IO | ✅ Done | Per-mount, gauges + sparklines |
| GPU (NVIDIA) | ✅ Done | All metrics including VRAM |
| Process tree | ✅ Done | Custom, collapsible |
| Process search/kill | ✅ Done | Live filter, SIGTERM |
| Clock + date + uptime | ✅ Done | Including moon phase |
| Calendar month view | ✅ Done | Keyboard-nav, today highlight |
| Music visualizer | ✅ Done | Animated, no audio capture |
| Media controls (MPRIS) | ✅ Done | playerctl integration |
| Battery | ✅ Done | Multi-BAT, time remaining |
| Dark/light theme | ✅ Done | Toggle with T key |
| Config-driven widgets | ✅ Done | TOML config file |
| **Summary bar** | ✅ Done | Single-line system overview |
| **Compact CPU** | ✅ Done | 2-per-row with inline temp/freq |
| **System Info widget** | ✅ Done | OS, WM, kernel, CPU, GPU, shell, terminal |
| **40/60 layout** | ✅ Done | Top metrics, full-width processes |
| **Process tree/flat indicator** | ✅ Done | [T]/[F] in status bar |
| Mouse support | ❌ Not done | Ratatui supports it via Crossterm events |
| Per-interface network | ❌ Not done | Shows aggregate only |
| AMD GPU support | ❌ Not done | sysinfo doesn't support it natively |
| Image previews | ❌ Not done | Needs Kitty protocol |
| Custom/scriptable widgets | ❌ Not done | |
| Process detail panel | ❌ Not done | Thread view, open files |
| Config persistence at runtime | ❌ Not done | Currently file-based only at launch |
| Responsive/compact layout detection | ❌ Not done | Fixed 40/60 split |
| blkio / cgroup monitoring | ❌ Not done | |
| Docker/container view | ❌ Not done | |

---

## Changelog (Recent)

- **2026-07-08** — Layout restructured to 40/60 (top metrics, full-width processes). CPU compacted to 2-per-row with inline temp/freq. Summary bar added. System Info widget added (OS, WM, kernel, CPU, GPU, shell, terminal). Process tree indicator added to status bar. Build: 0 warnings.

---

## Design Principles

1. **Single pane beats tabs.** The entire dashboard is visible at once. You don't flip between views — you glance.

2. **Terminal-native is the aesthetic.** No gradients, no rounded corners, no shadows. It uses terminal colors (RGB via crossterm), block characters (█░▒▓), box-drawing (│├─└), and the 24-bit color your terminal supports.

3. **Focus is explicit.** The focus border system makes it clear which panel will respond to keyboard input. Tab to move, Esc to release. The status bar confirms the focused panel name.

4. **Information density with whitespace.** Each panel is compact but readable. The 3-column grid uses all the horizontal space. The 40/60 layout prioritizes the most information-rich panel (processes) without sacrificing monitoring visibility.

5. **Real-time within terminal constraints.** The 0.5s refresh rate is fast enough for live monitoring but slow enough to avoid visual flicker in terminal emulators. Sparklines smooth out the update.

6. **No JS, no Web, no Electron.** This is a TUI. It runs in your terminal over SSH, in tmux, on a headless server. Zero web dependencies.

---

*Last updated: July 2026*
