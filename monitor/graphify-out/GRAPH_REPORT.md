# Graph Report - monitor  (2026-06-20)

## Corpus Check
- 37 files · ~16,732 words
- Verdict: corpus is large enough that graph structure adds value.

## Summary
- 657 nodes · 1302 edges · 46 communities (42 shown, 4 thin omitted)
- Extraction: 75% EXTRACTED · 25% INFERRED · 0% AMBIGUOUS · INFERRED: 331 edges (avg confidence: 0.58)
- Token cost: 0 input · 0 output

## Graph Freshness
- Built from commit: `2088e9b8`
- Run `git rev-parse HEAD` and compare to check if the graph is stale.
- Run `graphify update .` after code changes (no API cost).

## Community Hubs (Navigation)
- [[_COMMUNITY_Community 0|Community 0]]
- [[_COMMUNITY_Community 1|Community 1]]
- [[_COMMUNITY_Community 2|Community 2]]
- [[_COMMUNITY_Community 3|Community 3]]
- [[_COMMUNITY_Community 4|Community 4]]
- [[_COMMUNITY_Community 5|Community 5]]
- [[_COMMUNITY_Community 6|Community 6]]
- [[_COMMUNITY_Community 7|Community 7]]
- [[_COMMUNITY_Community 8|Community 8]]
- [[_COMMUNITY_Community 9|Community 9]]
- [[_COMMUNITY_Community 10|Community 10]]
- [[_COMMUNITY_Community 11|Community 11]]
- [[_COMMUNITY_Community 12|Community 12]]
- [[_COMMUNITY_Community 13|Community 13]]
- [[_COMMUNITY_Community 14|Community 14]]
- [[_COMMUNITY_Community 15|Community 15]]
- [[_COMMUNITY_Community 16|Community 16]]
- [[_COMMUNITY_Community 17|Community 17]]
- [[_COMMUNITY_Community 18|Community 18]]
- [[_COMMUNITY_Community 19|Community 19]]
- [[_COMMUNITY_Community 20|Community 20]]
- [[_COMMUNITY_Community 21|Community 21]]
- [[_COMMUNITY_Community 22|Community 22]]
- [[_COMMUNITY_Community 23|Community 23]]
- [[_COMMUNITY_Community 24|Community 24]]
- [[_COMMUNITY_Community 25|Community 25]]
- [[_COMMUNITY_Community 26|Community 26]]
- [[_COMMUNITY_Community 27|Community 27]]
- [[_COMMUNITY_Community 28|Community 28]]
- [[_COMMUNITY_Community 29|Community 29]]
- [[_COMMUNITY_Community 30|Community 30]]
- [[_COMMUNITY_Community 31|Community 31]]
- [[_COMMUNITY_Community 32|Community 32]]
- [[_COMMUNITY_Community 33|Community 33]]
- [[_COMMUNITY_Community 34|Community 34]]
- [[_COMMUNITY_Community 35|Community 35]]
- [[_COMMUNITY_Community 36|Community 36]]
- [[_COMMUNITY_Community 37|Community 37]]
- [[_COMMUNITY_Community 38|Community 38]]
- [[_COMMUNITY_Community 39|Community 39]]
- [[_COMMUNITY_Community 40|Community 40]]
- [[_COMMUNITY_Community 41|Community 41]]
- [[_COMMUNITY_Community 42|Community 42]]
- [[_COMMUNITY_Community 44|Community 44]]
- [[_COMMUNITY_Community 45|Community 45]]

## God Nodes (most connected - your core abstractions)
1. `SystemCollector` - 48 edges
2. `OverviewScreen` - 45 edges
3. `ProcessRow` - 39 edges
4. `ProcessService` - 37 edges
5. `HistoryBuffer` - 27 edges
6. `FileManagerScreen` - 23 edges
7. `GraphsScreen` - 22 edges
8. `ProcessTable` - 21 edges
9. `VantaMonitorTUI` - 20 edges
10. `SystemSnapshot` - 19 edges

## Surprising Connections (you probably didn't know these)
- `CPU` --uses--> `ProcessRow`  [INFERRED]
  tests/test_server.py → src/monitor/core/models.py
- `Memory` --uses--> `ProcessRow`  [INFERRED]
  tests/test_server.py → src/monitor/core/models.py
- `Network` --uses--> `ProcessRow`  [INFERRED]
  tests/test_server.py → src/monitor/core/models.py
- `ProcessRow` --uses--> `ProcessRow`  [INFERRED]
  tests/test_process_presenter.py → src/monitor/core/models.py
- `Path` --uses--> `VantaMonitorTUI`  [INFERRED]
  tests/test_dashboard_config.py → src/monitor/app.py

## Communities (46 total, 4 thin omitted)

### Community 0 - "Community 0"
Cohesion: 0.07
Nodes (24): Click, HistoryBuffer, ProcessRow, ProcessService, Screen, OverviewScreen, pal(), ProcessDetail (+16 more)

### Community 1 - "Community 1"
Cohesion: 0.19
Nodes (13): ArgumentParser, build_parser(), main(), Entry points for vanta-monitor CLI commands., Launch the Flask web dashboard., Launch the TUI + web dashboard simultaneously., Build the CLI argument parser., Main dispatcher for CLI subcommands. (+5 more)

### Community 2 - "Community 2"
Cohesion: 0.06
Nodes (28): get_palette(), is_light_theme(), next_theme_name(), Theme palettes and helpers for Vanta Monitor.  The TUI supports a small set of n, theme_label(), Key, main(), Vanta Monitor TUI app shell.  The shell owns screen registration, global keybind (+20 more)

### Community 3 - "Community 3"
Cohesion: 0.09
Nodes (46): load_dashboard_config(), build_calendar_widget(), build_clock_widget(), build_custom_text_widget(), build_fastfetch_widget(), build_image_widget(), build_matrix_widget(), build_music_widget() (+38 more)

### Community 4 - "Community 4"
Cohesion: 0.16
Nodes (29): BatterySnapshot, Collect per-device disk usage + per-device I/O rates + I/O busy., Aggregate + per-interface network counters., Battery via psutil.sensors_battery() — reads /sys/class/power_supply., Read battery power in watts from sysfs., CPU package power from Intel RAPL MSR (energy_uj delta)., _read_int(), _read_str() (+21 more)

### Community 5 - "Community 5"
Cohesion: 0.12
Nodes (36): MediaDetector, Media playback detection via playerctl., Return current playback info or *None* if nothing playing., Detect what media is currently playing via playerctl/MPRIS., _bar_color(), compact_bar(), _cpu_panel(), _disk_panel() (+28 more)

### Community 6 - "Community 6"
Cohesion: 0.11
Nodes (17): _pct_color(), ProcessSelected, ProcessTable, Reusable DataTable-based process list with keyboard actions., Return Rich color name for a utilization percentage., An auto-refreshing process DataTable with kill/suspend/resume., Emitted when a process action is triggered., Render status bar with sort indicator. (+9 more)

### Community 7 - "Community 7"
Cohesion: 0.11
Nodes (22): str, Screen-level tests for the Vanta dashboard and file manager., The hidden search box and user filter should affect screen state without crashin, Graphs screen should render CPU, memory, network, and disk trend panels., FileManager should show entries for the home directory., Pressing h should go to parent directory, l to enter a dir., Extract text content from a Static widget., The dashboard should render cpu, mem, net, disk, and system panels. (+14 more)

### Community 8 - "Community 8"
Cohesion: 0.10
Nodes (20): code:text (SystemCollector), code:bash (cd monitor), code:bash (graphify update .), `collectors.py`, Core modules, Current posture, File manager screen, Graphs screen (+12 more)

### Community 9 - "Community 9"
Cohesion: 0.22
Nodes (11): _build_dashboard_config(), DashboardConfig, _deep_merge(), ProcessConfig, Typed config loader for the modular Vanta dashboard., UIConfig, WidgetConfig, Any (+3 more)

### Community 10 - "Community 10"
Cohesion: 0.16
Nodes (19): auto_scale_range(), _format_bytes(), format_graph_header(), make_graph_label(), Helpers for formatting graph/trend data in terminal sparklines., time_range_label(), bool, float (+11 more)

### Community 11 - "Community 11"
Cohesion: 0.16
Nodes (15): format_process_detail(), format_process_status(), _pct_color(), Formatting helpers for process lists and detail views.  All colour-producing fun, bool, float, int, ProcessRow (+7 more)

### Community 12 - "Community 12"
Cohesion: 0.13
Nodes (14): Architecture, code:bash (cd monitor), code:json ({), code:text (src/monitor/), code:bash (cd monitor), Config, Global, Keybinds (+6 more)

### Community 13 - "Community 13"
Cohesion: 0.15
Nodes (12): FileManagerScreen, _fmt_size(), _fmt_time(), _icon_for(), pal(), Simple icon based on file type., Keyboard-driven file browser: navigate, preview, open., ComposeResult (+4 more)

### Community 14 - "Community 14"
Cohesion: 0.18
Nodes (10): audio, sensitivity, simulated, process, auto_refresh, max_display, show_kernel, ui (+2 more)

### Community 15 - "Community 15"
Cohesion: 0.18
Nodes (10): audio, sensitivity, simulated, process, auto_refresh, max_display, show_kernel, ui (+2 more)

### Community 16 - "Community 16"
Cohesion: 0.19
Nodes (18): _bool_arg(), get_gpu_stats(), _int_arg(), kill_process(), load_config(), process_detail(), processes(), Get GPU utilization, memory, and temperature. (+10 more)

### Community 17 - "Community 17"
Cohesion: 0.25
Nodes (8): enabled, enabled, max_depth, enabled, widgets, calendar, pstree, system_stats

### Community 18 - "Community 18"
Cohesion: 0.25
Nodes (8): enabled, enabled, max_depth, enabled, widgets, calendar, pstree, system_stats

### Community 20 - "Community 20"
Cohesion: 0.50
Nodes (4): enabled, format, show_date, clock

### Community 21 - "Community 21"
Cohesion: 0.50
Nodes (4): density, enabled, speed, matrix

### Community 22 - "Community 22"
Cohesion: 0.50
Nodes (4): bars, enabled, sensitivity, music_viz

### Community 23 - "Community 23"
Cohesion: 0.50
Nodes (4): enabled, max_display, show_kernel, process_manager

### Community 24 - "Community 24"
Cohesion: 0.50
Nodes (4): directory, enabled, interval, wallpaper

### Community 25 - "Community 25"
Cohesion: 0.50
Nodes (4): enabled, format, show_date, clock

### Community 26 - "Community 26"
Cohesion: 0.50
Nodes (4): density, enabled, speed, matrix

### Community 27 - "Community 27"
Cohesion: 0.50
Nodes (4): bars, enabled, sensitivity, music_viz

### Community 28 - "Community 28"
Cohesion: 0.50
Nodes (4): enabled, max_display, show_kernel, process_manager

### Community 29 - "Community 29"
Cohesion: 0.50
Nodes (4): directory, enabled, interval, wallpaper

### Community 30 - "Community 30"
Cohesion: 0.67
Nodes (3): enabled, sections, custom_text

### Community 31 - "Community 31"
Cohesion: 0.67
Nodes (3): enabled, path, image

### Community 32 - "Community 32"
Cohesion: 0.67
Nodes (3): columns, enabled, dashboard

### Community 33 - "Community 33"
Cohesion: 0.67
Nodes (3): enabled, refresh_interval, fastfetch

### Community 34 - "Community 34"
Cohesion: 0.67
Nodes (3): yazi, cwd, enabled

### Community 35 - "Community 35"
Cohesion: 0.67
Nodes (3): enabled, sections, custom_text

### Community 36 - "Community 36"
Cohesion: 0.67
Nodes (3): enabled, path, image

### Community 37 - "Community 37"
Cohesion: 0.67
Nodes (3): columns, enabled, dashboard

### Community 38 - "Community 38"
Cohesion: 0.67
Nodes (3): enabled, refresh_interval, fastfetch

### Community 39 - "Community 39"
Cohesion: 0.67
Nodes (3): yazi, cwd, enabled

### Community 44 - "Community 44"
Cohesion: 0.06
Nodes (20): callable, looks_like_kernel(), next_sort_column(), prev_sort_column(), Heuristic: kernel threads start with a known prefix or contain '/'., Return a key function for a column name (cpu->cpu_percent, etc)., Return detailed info about a process., Return detailed info about a process. (+12 more)

### Community 45 - "Community 45"
Cohesion: 0.20
Nodes (6): CPU, _FakeSnapshot, Memory, Network, Tests for the Flask web dashboard and API surface., test_stats_endpoint_returns_snapshot_payload()

## Knowledge Gaps
- **105 isolated node(s):** `refresh_rate`, `theme`, `show_kernel`, `max_display`, `auto_refresh` (+100 more)
  These have ≤1 connection - possible missing edges or undocumented components.
- **4 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `OverviewScreen` connect `Community 0` to `Community 2`, `Community 3`, `Community 4`, `Community 7`?**
  _High betweenness centrality (0.224) - this node is a cross-community bridge._
- **Why does `ProcessRow` connect `Community 0` to `Community 4`, `Community 5`, `Community 11`, `Community 44`, `Community 45`?**
  _High betweenness centrality (0.135) - this node is a cross-community bridge._
- **Why does `ProcessService` connect `Community 0` to `Community 16`, `Community 44`, `Community 6`?**
  _High betweenness centrality (0.130) - this node is a cross-community bridge._
- **Are the 34 inferred relationships involving `SystemCollector` (e.g. with `str` and `Any`) actually correct?**
  _`SystemCollector` has 34 INFERRED edges - model-reasoned connections that need verification._
- **Are the 11 inferred relationships involving `OverviewScreen` (e.g. with `VantaMonitorTUI` and `str`) actually correct?**
  _`OverviewScreen` has 11 INFERRED edges - model-reasoned connections that need verification._
- **Are the 38 inferred relationships involving `ProcessRow` (e.g. with `float` and `str`) actually correct?**
  _`ProcessRow` has 38 INFERRED edges - model-reasoned connections that need verification._
- **Are the 30 inferred relationships involving `ProcessService` (e.g. with `str` and `Any`) actually correct?**
  _`ProcessService` has 30 INFERRED edges - model-reasoned connections that need verification._