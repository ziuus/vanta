# Vanta Monitor — UI Gap Analysis vs btop

**Date:** 2026-07-01  
**Scope:** All 5 Textual TUI screens + CSS stylesheets + supporting modules  
**Reference:** btop v1.3+ (feature set derived from [btop README](https://github.com/aristocratos/btop) and community documentation)

---

## Table of Contents

1. [Overview Screen](#1-overview-screen)
2. [Graphs Screen](#2-graphs-screen)
3. [Widgets Screen](#3-widgets-screen)
4. [File Manager Screen](#4-file-manager-screen)
5. [Help Overlay](#5-help-overlay)
6. [Cross-Cutting Gaps](#6-cross-cutting-gaps)
7. [Prioritized Remediation Roadmap](#7-prioritized-remediation-roadmap)

---

## 1. Overview Screen

**Files:** `screens/overview.py`, `styles/overview.tcss`, `core/overview_presenter.py`  
**Quality Level: 6/10 — Functional, operator-focused, but visually sparse vs btop**

### What Works Well ✅

| Feature | Detail |
|---|---|
| CPU panel | Total % bar, per-core bars (2-col grid), per-core temps, frequency, load avg, package power |
| Memory panel | Used/total/available bytes, swap %, cached/buffers/slab breakdown |
| Network panel | Aggregate up/down rates, total bytes, top 2 interfaces with per-iface rates |
| Disk panel | Per-mount percent bars, free space, per-device IO rates, IO busy % |
| GPU panel (in system strip) | NVML-powered: util%, temp, power, clock speeds, VRAM, encoder/decoder |
| Battery | Percent, status (Charging/Discharging), time remaining, power draw |
| Process list | Flat + tree view, sortable (5+ cols), PID search, kernel/user filter |
| Process detail | Modal overlay showing 17 fields (exe, cmdline, cwd, env, threads, connections, FDs, CPU affinity) |
| Signal menu | Modal list of 8 POSIX signals with descriptions, callback-based |
| Media now-playing | MPRIS detection with animated equalizer bars, play/pause/next/prev controls |
| Compact/tiny modes | 3-level responsive layout via height thresholds |
| Error reporting | Error capture in status strip with red marker |
| Sparklines in status strip | 8-char spark bars for CPU/MEM/DISK atop the overview |
| Mouse click-to-select | Click on process panel row to select |
| Theme-aware colors | All panels use palette colors consistently |

### What's Missing/Weak vs btop 🔴

| Gap | Severity | btop Equivalent | Concrete Changes Needed |
|---|---|---|---|
| **No process tree expand/collapse** | High | Interactive tree with `[+]/[-]` toggles per process, smooth animation | Replace `_tree_rows()` static depth walk with per-process expand/collapse state map; toggle on keypress; animate indentation |
| **No mouse scroll on process list** | High | Scrollwheel moves selection; scrollbar visible | Add `on_mouse_scroll_down/up` handlers; compute visible window from scroll offset |
| **No click-to-sort column headers** | High | Click any column header to sort asc/desc | Make `_process_header()` clickable by rendering each column as a separate clickable region with `on_click` bound to column key |
| **No kill confirmation dialog** | Medium | Confirmation box before kill with countdown | Add `KillConfirm` screen similar to `SignalMenu` but with "Confirm KILL?" + cancel |
| **No configurable process columns** | Medium | User toggles columns via key/UI | Make process columns a config list in `dashboard_config`; filter header/render based on enabled columns |
| **No aggregate process stats footer** | Medium | Total CPU%, total MEM%, thread count, running/sleeping counts | Compute aggregates from `_processes` list and append to `_process_footer()` |
| **No cumulative per-process disk IO rates** | Medium | btop shows read/write speeds per process | Already partially there but IO rates are cumulative bytes, not rates; convert to Bps with delta tracking |
| **No load average sparkline in panel** | Medium | Mini sparkline embedded in CPU panel | Add `_load_hist` HistoryBuffer and sparkline render next to load avg text |
| **GPU panel is a narrow strip, not a real panel** | Medium | GPU gets its own resizable panel with full bar+temp+clock display | Create dedicated GPU panel `#gpu-panel` in the overview layout, sibling to network/disk |
| **No disk activity sparkline per device** | Low | Small inline spark for each disk IO | Track per-device IO histograms (may need HistoryBuffer dictionary keyed by device name) |
| **No per-core frequency display** | Low | btop shows individual core frequencies if available | `per_core_freqs` currently not collected; add `_sample_per_core_freqs()` using `/sys/devices/system/cpu/*/cpufreq/scaling_cur_freq` |
| **No container/cgroup names** | Low | Container runtime detection, shows container name next to PID | Add cgroup v2 parsing to process service; map PID → container name |
| **No "hold" / freeze mode** | Medium | Space bar freezes display to read values | Add `_frozen` flag; skip collector calls when frozen; toggle with space keybinding |
| **Network interface selector** | Low | User cycles through which interfaces to display in main panel | Add `_selected_iface` index and cycle keys; render only that interface's data in network panel |

### CSS/Visual Polish Issues

| Issue | Fix |
|---|---|
| Cramped padding on small terminals | `overview.tcss` uses `padding: 0 1` with no dynamic sizing; switch to min-content-based padding |
| Panel borders use `solid $border` — no variety | Use different border styles per panel type (`hkey`, `round`, etc.) and colored left-edge accents |
| No alternating row colors in process list | Add CSS class `.proc-row-even` / `.proc-row-odd` with subtle background tint |
| Selected process row highlight is just a `▸` character | Could render full-row highlight via CSS `.--highlight` on a ListView, or use Rich `reverse` styling |
| Header/Footer chrome takes too much space on small terminals | Hide Footer in compact mode; shrink Header to single-line in tiny mode |

---

## 2. Graphs Screen

**Files:** `screens/graphs.py`, `styles/graphs.tcss`, `core/graph_presenter.py`, `core/history.py`  
**Quality Level: 3/10 — Minimal sparkline-only implementation; major gap vs btop**

### What Works Well ✅

| Feature | Detail |
|---|---|
| Dual buffer sizes | Short (60 samples) + long (300 samples) per metric |
| Four metric panels | CPU %, Memory %, Network throughput, Disk I/O |
| Color-coded current values | Green < 50%, Yellow < 80%, Red ≥ 80% |
| Sparkline rendering | Unicode block chars (`▁▂▃▄▅▆▇█`) with auto-scaled range |
| Min/mid/max labels | `make_graph_label()` shows lower/upper bounds |
| Time range labels | `time_range_label()` shows "1m" / "5m" |
| Responsive compact/tiny modes | Height-based padding reduction, auto-sizing |

### What's Missing/Weak vs btop 🔴

| Gap | Severity | btop Equivalent | Concrete Changes Needed |
|---|---|---|---|
| **No real line/area graph rendering** | **Critical** | btop draws proper line charts using `─│┌┐└┘├┤┼╭╮╰╯` with filled area blocks | Replace `_spark()` with a proper graph renderer that plots X/Y data using braile dots (`⣀⣤⣶⣿`) or block-based area fills; implement Y-axis range auto-tick |
| **No zoom/scroll through history** | High | Arrow keys scroll graph left/right; +/- zooms time range | Add `_graph_offset` and `_graph_zoom` state; keybindings for zoom in/out and pan left/right; clip data window to viewport |
| **Only 2 time scales (1m + 5m)** | High | btop shows 30s / 5m / 15m with proportional zoom | Add 3rd buffer (15m = 900 samples at 1s); make ranges selectable via hotkeys |
| **No per-core CPU graphs** | Medium | Graph each core individually, toggleable | Add per-core `HistoryBuffer` dict; render stacked or overlaid per-core lines |
| **No mouse interaction at all** | High | Hover shows value at point; click sets zoom anchor; scrollwheel zooms | Add `on_mouse_move` to compute hovered data point; `on_click` for zoom anchor; `on_mouse_scroll` for zoom |
| **No graph smoothing** | High | Moving-average smoothing with configurable window | Pre-process data with SMA/EMA before rendering; add smoothing level config |
| **No per-interface network graphs** | Medium | Each interface has its own graph line | Add `_iface_hist` dict keyed by interface name; render selected interface's graph |
| **No per-device disk IO graphs** | Medium | Individual device read/write graphs | Currently only tracks first disk; change to per-device HistoryBuffer keyed by device |
| **No temperature history** | Medium | CPU/GPU temperature graph with min/max markers | Add temp HistoryBuffer in graph screen; render as overlay or separate panel |
| **No custom time range selection** | Medium | User types a range (e.g. "10m") | Add input prompt for custom range; resize buffers dynamically |
| **No graph panel resize** | Medium | Panels are resizable with mouse drag | Make graph panels Textual `Vertical` with `weight` implementation; store size ratio |
| **No mini-graphs in overview panels** | High | btop embeds sparkline graphs directly in CPU/Mem/Net/Disk overview panels | Embed 20-character sparkline in each overview panel header, driven by the overview's own HistoryBuffers |
| **No axis labels / grid lines** | Low | btop shows Y-axis scale labels on the left | Add Y-axis tick labels (e.g., "0", "50", "100") along left margin |
| **Graph panel height is too generous** | Low | btop auto-sizes graphs to fill available space | Currently `height: 1fr` which works but with header/footer trimming leaves ~8 rows per graph; enforce minimum 5 rows for readability |
| **No graph title customisation** | Low | Each graph panel title is configurable | Read titles from `dashboard_config` or allow per-graph setting |

### CSS/Visual Polish Issues

| Issue | Fix |
|---|---|
| No border differentiation between short/long graphs | Both sparklines look the same; use a subtle `#` symbol or dim/bright distinction |
| Graph panel headers are plain text, not styled | Use `.panel-title` class or equivalent for bold accent-colored header |
| No loading state | When buffers are cold (< 5 samples), show "collecting..." instead of misleading spark |
| `graphs.tcss` doesn't force monospace font | Add `font-family: monospace` to `.graph-panel` for consistent alignment |

---

## 3. Widgets Screen

**Files:** `screens/widgets_screen.py`, `styles/widgets.tcss`  
**Quality Level: 2/10 — Mostly cosmetic/decorative; no real monitoring value vs btop**

### What Works Well ✅

| Feature | Detail |
|---|---|
| Clock widget | Renders time via `WidgetRenderCache` |
| Calendar widget | Renders month calendar |
| Matrix rain effect | Terminal "Matrix" digital rain animation |
| Music visualizer | Animated sine-wave equalizer bars |
| Fastfetch widget | System info (distro, kernel, uptime, packages, etc.) |
| Config-driven | Each widget can be enabled/disabled in JSON config |
| Widget render caching | `WidgetRenderCache` avoids recomputing static content |
| Theme-aware | Respects palette colors |

### What's Missing/Weak vs btop 🔴

| Gap | Severity | btop Equivalent | Concrete Changes Needed |
|---|---|---|---|
| **No CPU/Mem/Net/Disk live mini-dashboard** | High | btop's main screen IS the dashboard. The widgets screen adds no monitoring value | Add live system stat widgets: real-time CPU% gauge, MEM bar, NET up/down, disk usage — updated every refresh tick |
| **No process tree widget** | High | "pstree" is configured in `dashboard_config` but never rendered | Check `_is_enabled("pstree")` and render a mini process tree in a framed widget |
| **No disk usage tree/map** | Medium | btop has a disk usage map showing directory sizes | Add `dutree`-inspired widget showing dir sizes with proportional bars |
| **No network connection list** | Medium | btop shows active connections (like `ss`) | Add widget rendering `ss -tuln` output or use psutil.net_connections() |
| **No sensor list widget** | Medium | btop lists all sensors (temps, fans, voltages) | Add widget that iterates `psutil.sensors_*()` and renders a table |
| **No custom command widget** | Medium | btop doesn't have this, but for Vanta it's a missed opportunity — user-defined shell commands displayed inline | Already partially supported via `custom_text` config; extend to run shell commands and display stdout |
| **Matrix effect is pure cosmetic** | Low | N/A (btop has no equivalent) | Keep as decoration but ensure it doesn't consume CPU; add throttling |
| **Music viz is simulated, not real** | Low | N/A (btop has no music viz) | Real audio capture would require audio library deps; current sin-based animation is acceptable for a decorative widget but document the limitation |
| **No interactive widgets** | Medium | btop panels are interactive | Make clock clickable to toggle format (12h/24h); calendar clickable to show events |
| **Layout is hardcoded in CSS** | High | btop's layout is fully configurable with per-panel positioning | Use a grid layout system driven by config; allow users to arrange widgets via config file |
| **No wallpaper/slideshow widget rendered** | Low | Wallpaper widget is configured but disabled by default and no rendering code in widgets screen | Add render path for `wallpaper` widget (requires image rendering in terminal — very advanced) |

### CSS/Visual Polish Issues

| Issue | Fix |
|---|---|
| Widgets are just text blocks stacked vertically | Each widget should have a bordered frame with accent-colored title bar |
| No gap control between widgets | `blocks.append("")` adds empty lines; use proper CSS margin instead |
| Matrix widget has no size constraints | Matrix can grow unbounded; cap height and clip overflow |
| No loading/empty state for disabled widgets | Show "disabled" placeholder or hide entirely (currently hidden by `_is_enabled()` check which is correct) |

---

## 4. File Manager Screen

**Files:** `screens/filemanager.py`, `styles/filemanager.tcss`  
**Quality Level: 5/10 — Functional basic file browser, but limited feature set**

### What Works Well ✅

| Feature | Detail |
|---|---|
| Vim-style navigation | `j`/`k` up/down, `h` parent, `l` enter, `g`/`G` top/bottom |
| Directory listing | Sorted: dirs first, then alphabetical |
| Icon per file type | ~30 file extension → emoji mappings |
| File preview pane | Metadata display for selected file (size, mtime) |
| Directory preview | Shows subdirectory counts (files/dirs) |
| Path display with icon | Current working directory shown at top |
| Item count status bar | Shows "N items" |
| Theme-aware | Colors from palette |
| Home shortcut | `~` jumps to `$HOME` |
| Cursor restoration | When going to parent dir, cursor restores to previously-visited directory |
| ListView-based | Built on Textual's native ListView for keyboard nav |

### What's Missing/Weak vs btop 🔴

| Gap | Severity | btop Equivalent | Concrete Changes Needed |
|---|---|---|---|
| **No file content preview** | **Critical** | btop has no file manager, but a good TUI FM (ranger/nnn) has syntax-highlighted text preview | Add text file preview (first 200 lines with syntax highlighting via Pygments or Rich); binary file hexdump; image → sixel/iTerm protocol |
| **No file operations** | High | ranger/nnn: copy (y), paste (p), delete (dd), rename | Add `action_copy`, `action_paste`, `action_delete`, `action_rename` with confirmation dialogs |
| **No bulk/multi-select** | Medium | Space to select, visual mode (v) for range | Add `_selected_set: set[int]` for multi-select; visual mode toggle |
| **No search within directory** | High | `/` is bound in BINDINGS but `action_search` is missing | Implement `action_search` — filter-as-you-type Input overlay that filters `_entries` |
| **No sort toggling** | Medium | Sort by name/size/date/mtime with keybinds | Add sort mode enum (`NAME`, `SIZE`, `MTIME`, `TYPE`); keybinding to cycle |
| **No hidden file toggle** | Medium | `.` toggle to show/hide dotfiles | Add `_show_hidden` flag; filter entries in `_load_dir()` |
| **No directory size calculation** | Medium | `du -sh` on selected dir | Add `_calc_dir_size()` with spinner/async; cache results |
| **No permissions display** | Medium | rwx mode string | Add permission column using `stat().st_mode` |
| **No owner/group display** | Medium | User and group names | `stat().st_uid` → `pwd.getpwuid()`, `stat().st_gid` → `grp.getgrgid()` |
| **No symlink target display** | Low | `-> /target/path` for symlinks | `os.readlink()` appended to name |
| **No path auto-completion** | Low | Tab-complete partial paths | Extend search Input to support tab-completion against filesystem |
| **200-entry cap is arbitrary** | Low | Show all entries with virtual scrolling | Remove cap; implement lazy-loading or virtual scroll for directories with >1000 entries |
| **No file type indicators** | Low | `|` for pipe, `=` for socket, `%` for door | Extend `_icon_for()` or add type column |
| **No cut/copy buffer** | Medium | Clipboard for file operations | Add `_cut_buffer: list[Path]` and `_copy_buffer: list[Path]` |
| **No undo** | Low | Undo last file operation | Keep operation history deque |
| **No file filter by type** | Medium | Show only dirs, or only images, etc. | Add `_type_filter` with modes: all/dirs/files/images |
| **No "open with" functionality** | Low | External program launcher | Add `action_open_with` that prompts for command; use `subprocess.Popen` |

### CSS/Visual Polish Issues

| Issue | Fix |
|---|---|
| Selected row highlight uses ListItem's `--highlight` class but no custom styling | Add more visible highlight (accent background vs subtle surface-alt) |
| Preview pane has no scroll | Content may overflow; add `overflow-y: auto` |
| No column alignment for size/mtime | Currently rendered manually with f-string padding; use Textual DataTable or formatted Static with monospace |
| Item count is plain text, not styled | Add accent-colored count number |

---

## 5. Help Overlay

**Files:** `screens/help_screen.py`, `styles/help.tcss`  
**Quality Level: 7/10 — Clean, functional, good design — minor gaps**

### What Works Well ✅

| Feature | Detail |
|---|---|
| Clean modal overlay | Semi-transparent background, centered content, border accent |
| Categorized keybindings | Navigation, Overview, General categories with separator lines |
| Theme-aware | Uses palette for all colors |
| Multiple dismiss paths | Esc, ?, q all close |
| Global + screen-specific binds | Documents both app-level and per-screen actions |
| Version/theme label in title | Shows current theme name |

### What's Missing/Weak vs btop 🔴

| Gap | Severity | btop Equivalent | Concrete Changes Needed |
|---|---|---|---|
| **No search within help** | Low | btop uses standard help | Add Input filter to search binding descriptions |
| **No context-sensitive help** | Low | btop shows bottom-bar key hints per screen | Vanta already shows some hints in process footer; extend to all screens via a `help_hints` property |
| **No mouse action documentation** | Low | N/A (btop is keyboard-first too) | Add a "Mouse" section documenting click-to-sort, click-to-select etc. once those features exist |
| **No config reference** | Medium | btop help includes option descriptions | Add section showing current config values (refresh rate, theme, enabled widgets) |
| **No dynamic content** | Low | Static list | Dynamically generate keybinding list from actual registered bindings rather than hardcoded `KEYBINDS` |
| **No "what's new" / changelog** | Low | N/A | Add version-based changelog section |
| **Hardcoded keybind list** | Medium | btop reads bindings from config | Populate from app/walked screen bindings programmatically |

### CSS/Visual Polish Issues

| Issue | Fix |
|---|---|
| Help modal width is fixed at 60 | Should be responsive: `width: 60%` with min/max constraints |
| Category separators use `border-top: solid` | Should use softer separator char (like `─ ─ ─`) |
| No scroll if help overflows screen | Add `max-height: 80%` and `overflow-y: auto` to `#help-modal` |
| Key column width is 10, may clip long key combos | Use `min-width` + `auto` sizing |

---

## 6. Cross-Cutting Gaps

### 6.1 Visual Polish & Graph Quality

| Gap | Current State | btop State | Fix |
|---|---|---|---|
| Graph rendering | Unicode bar chars (`▁▂▃▄▅▆▇█`) only | Box-drawing line/area charts with filled regions | Implement proper line graph renderer using braille dots or half-block fills |
| Gauge rendering | `█·` chars with fixed width | Full-block unicode gauges with color gradients | Add gradient support; use `█` + `▓` + `▒` + `░` for partial fills |
| Color depth | 8 named + hex from 4 palettes | 256-color + truecolor with color uniformity ensures | Ensure all rendered strings use hex colors from palette, not named colors |
| Process highlighting | `▸` character prefix only | Full-row reverse-color highlight, alternating rows | Add CSS class-based row highlighting; use Rich `on` style for selection |
| Borders / frames | Textual `solid` borders | Rounded corners (`╭╮╰╯`) with shadow | Replace with custom Rich border renderers using box-drawing chars |
| Animation / transitions | None | Smooth transition between states (CPU%, graph data) | Add interpolation between values; use `set_interval` with delta-time for smooth rendering |
| Unicode glyph consistency | Mixed emoji (🖥🧠🌐💾) vs text | Clean monospace glyphs (uses Font Awesome icons in some configs) | Decide on icon set; replace emoji with NF (Nerd Font) icons or simple text markers |

### 6.2 Mouse Support

| Gap | Current State | btop State | Fix |
|---|---|---|---|
| Click-to-sort | Not implemented | Click any column header | Add click handlers per column region in process header |
| Scroll-to-navigate | Not implemented | Scrollwheel moves selection | Add `on_mouse_scroll_down/up` to process list and file list |
| Panel resize | Not implemented | Drag panel borders with mouse | Implement Textual Vertical with draggable dividers (complex — requires custom widget) |
| Right-click context menu | Not implemented | Right-click on process for signal/strace | Add `on_mouse_down` with button=2 to show context menu |
| Hover tooltips | Not implemented | Hover on graph shows value | Add `on_mouse_move` handler for graph panels |
| Click-to-focus panel | Not implemented | Click any panel to make it active | Added implicitly if all panels become focusable |

### 6.3 Input Handling

| Gap | Current State | btop State | Fix |
|---|---|---|---|
| Key repeat | Only works when Textual handles it natively | Full key repeat with configurable rate | Rely on Textual's built-in key repeat; test across terminals |
| Incremental process search | Requires `/` → type → Enter | Type PID or name directly with auto-filter | Implement "type-to-filter" mode that intercepts alphanumeric keys as search |
| PID jump | Not supported | Type PID number → Enter to select | Add numeric mode where digits build up a PID then jump to it |
| Tab completion | Not supported | Tab completion in search | Add to search Input widget |
| Undo for operations | Not supported | N/A | Keep command history for process operations (signal sending) |

### 6.4 Responsive Layout

| Gap | Current State | btop State | Fix |
|---|---|---|---|
| Layout modes | 3 hardcoded (full/compact/tiny) | Fully dynamic per-panel sizing | Add configurable panel weights; respect width as well as height |
| Minimum sizes | `min-height` in CSS | Per-panel min/max with enforcement | Add runtime min-size enforcement with overflow prevention |
| Custom layout | Not supported | Users define panel positions in config | Add "layout" section to `dashboard_config` with per-panel position/size |
| Width awareness | Height-based only | Both width and height adaptive | Add width-based layout thresholds; collapse sidebar panels on narrow terminals |

### 6.5 Theme Depth

| Gap | Current State | btop State | Fix |
|---|---|---|---|
| Theme count | 4 presets (light, dark, monokai, nord-light) | 20+ built-in presets | Port popular btop themes (tty, gruvbox, dracula, catppuccin, tokyo-night, etc.) |
| Custom theme creation | Code-only, requires Python changes | In-theme editing via GUI | Add config-file-based theme overrides; implement theme picker screen |
| Per-element theming | Global palette only | Individual element colors (proc color, graph color, etc.) | Extend palette to include element-specific keys (e.g. `proc_cpu`, `graph_line`) |
| Background image | Not supported | PNG/Sixel background with transparency | Requires terminal graphics protocol support — very advanced, defer |
| Transparency | CSS `rgba()` used in overlays | Theme-defined transparency per element | Expand palette with alpha-channel color keys |
| Truecolor fallback | No handling | Automatic 256-color fallback | Add terminal color detection and color translation |

### 6.6 Performance & Robustness

| Gap | Current State | btop State | Fix |
|---|---|---|---|
| Timing precision | `time.time()` + `set_interval(1.0)` | `clock_gettime(CLOCK_MONOTONIC)` with microsecond precision | Switch to `time.monotonic_ns()` / `perf_counter_ns()` for delta calculations |
| Frame rate | Locked to refresh interval (1s default) | Configurable FPS cap with vsync-aware tick | Use Textual's `set_interval` at a higher rate (16ms for 60fps) for smooth animations; decouple data sampling from rendering |
| Adaptive refresh | Not implemented | Adjusts rate based on activity | Increase refresh during high CPU/IO activity; decrease when idle |
| Async collectors | Synchronous psutil calls | Async collectors via thread pool | Wrap psutil calls in `asyncio.to_thread()` or ThreadPoolExecutor |
| Error resilience | Error skips entire update | Shows stale data with error indicator | Show last known good values plus "stale" flag instead of blank screen |
| Memory usage | HistoryBuffers are unbounded in growth | Ring buffers with fixed maxlen | Already using `deque(maxlen=N)` — good |
| CPU overhead | Always at 100% polling rate | Adaptive polling with idle reduction | Reduce poll rate when terminal is obscured or no changes detected |

---

## 7. Prioritized Remediation Roadmap

### Phase 1 — Immediate Wins (1–2 days)
1. **Mouse scroll on process list** — Add `on_mouse_scroll_down/up` to Overview
2. **Click-to-sort column headers** — Make process header columns clickable
3. **Process freeze mode** — Add space bar toggle for pause/resume
4. **Graph smoothing** — Add SMA/EMA pre-processing to `_spark()`
5. **Help modal scroll** — Fix overflow with `max-height` and `overflow-y: auto`
6. **Kill confirmation dialog** — Add confirmation screen before sending KILL
7. **Hidden file toggle** — Add `.` toggle to file manager
8. **Process search in file manager** — Implement `action_search` (bound, not coded)

### Phase 2 — Graph Revolution (3–5 days)
1. Replace `_spark()` with box-drawing line graph renderer (braille dots or half-blocks)
2. Add zoom/scroll (arrow keys for pan, +/- for zoom)
3. Add 3rd time scale (15m = 900 samples)
4. Per-core CPU history line graphs
5. Temperature history tracking and rendering
6. Mini sparklines embedded in overview panel headers

### Phase 3 — Theme Expansion (2–3 days)
1. Port 10+ btop theme presets (gruvbox, dracula, catppuccin, tokyo-night, solarized, etc.)
2. Add config-file-based theme override mechanism
3. Extend palette with element-specific color keys
4. Add theme preview/picker screen

### Phase 4 — Advanced Interaction (3–5 days)
1. Process tree expand/collapse with state tracking
2. Full file manager enhancements (multi-select, copy/paste, delete, rename)
3. Grid layout system for widgets screen with user-configurable layout
4. Mouse-drag panel resizing
5. Right-click context menu for processes

### Phase 5 — Performance & Polish (2–3 days)
1. Async collector wrappers
2. Delta-time based smooth rendering
3. Truecolor fallback detection
4. Adaptive refresh rate
5. Footer/hints per screen
6. Dynamic help keybinding list

---

## Summary Statistics

| Metric | Vanta Monitor | btop |
|---|---|---|
| Theme presets | 4 | 20+ |
| Graph rendering | Unicode bar sparklines only | Box-drawing line/area charts |
| Mouse support | Click-to-select (1 gesture) | Full: sort, scroll, resize, context menus |
| Process tree | Static flat indent per process | Interactive expand/collapse per process |
| Time scales | 2 (1m, 5m) | 3+ (30s, 5m, 15m) + custom |
| Zoom/pan | Not supported | Full zoom/scroll through history |
| Smoothing | None | Moving average |
| Panel resizing | Not supported | Mouse-drag dividers |
| Layout | 3 hardcoded modes | Fully dynamic/configurable |
| File manager | Basic browse + metadata preview | N/A (but comparable to ranger-lite) |
| GPU support | NVML only (NVIDIA) | NVIDIA + AMD ROCm |
| Container awareness | None | cgroup v2 detection |
| Timing precision | ~1s (set_interval) | Microsecond (clock_gettime) |
| Keybindings | 20+ across screens | 50+ |

---

*Document prepared by Hermes Agent — based on complete source audit of `~/Projects/vanta/monitor/`*
