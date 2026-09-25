# Vanta Architecture & Agent Guide

This document is intended for AI agents and developers working on the Vanta codebase. It outlines the architectural boundaries, rendering pipelines, and extension mechanisms.

## 1. Core Philosophy
Vanta is a highly-optimized, aesthetic terminal dashboard written in Rust using `ratatui`. 
* **Performance first**: The main render loop must never block on I/O. All data fetching (monitors, feeds, filesystem operations) happens in background threads that update a shared state (`Summary`, `App` states, or `fs_tasks`).
* **Terminal limits**: UI components must gracefully handle terminal resizing and tiny viewports.
* **No bloat**: Core features should be universally useful. Niche features belong in WASM extensions.

## 2. Rendering Pipeline
* **`src/main.rs`**: The entry point. Initializes the terminal, loads the config, boots background workers, and drives the TUI event loop.
* **`src/app.rs`**: Holds the global state (`App`). Contains the navigation (`DashboardMode`) and handles all keystrokes. Keystrokes on custom pages are routed to active WASM extensions via `handle_key`.
* **`src/screens/dashboard.rs`**: The primary layout engine. It takes the `[[pages.layout]]` grid from the config and dynamically maps string IDs to their rendering functions.

## 3. Configuration (`src/config.rs`)
Vanta is driven by `config.toml`. 
* **Custom Pages**: Users define `[[pages]]` with layouts representing a grid of component IDs.
* **Extensions**: The `[extensions]` table controls which compiled WASM extensions are booted.

## 4. Extension Architecture (V2 WASM Micro-Extensions)
Vanta uses Extism for sandboxed `wasm32-unknown-unknown` plugins. The core API is in `src/extension/host_api.rs`.
* **Micro-Extension Pattern**: We enforce a 1-to-1 mapping where possible. Every individual widget (e.g., `filespace_browser`, `filespace_preview`) is compiled as its own independent `.wasm` plugin.
* **State Isolation & Communication**: Extism sandboxes cannot share memory. To allow micro-extensions to communicate (e.g., a browser telling a preview pane what file is selected), Vanta provides a host-side Key-Value mailbox.
  * Extensions call `vanta_query` with `{"topic": "state_set", "key": "...", "value": ...}` to broadcast state.
  * Other extensions call `vanta_query` with `{"topic": "state_get", "key": "..."}` to read it.
* **Host API Boundaries**: WASM plugins cannot perform blocking I/O (no `std::fs`, no threading). They must query the host for JSON snapshots (e.g., `fs_list`, `fs_ops`, `media`). Heavy tasks must be dispatched to the host via `fs_action` so they run on background threads.

## 5. Frame Scheduling & Idle Budget
Vanta is meant to run 24/7, so a static screen must idle at ~2 fps (~4% of a core on an i5-8265U).
* `src/anim.rs`: widgets that visibly animate call `anim::request(fps)` / `anim::request_full()` during render. The event loop redraws at the highest request (capped by `ui.fps`), else `anim::IDLE_FPS`. Input and resizes redraw immediately.
* Never animate on a silent/idle state at full fps. Only request frames while there's actual motion.
* Samplers whose data only extensions read (`services`, `connections`) use `monitors::Demand` so they run only while someone reads them.
* Profile with `VANTA_PROFILE=1` (writes `/tmp/vanta-profile.log`).

## 6. Build & Verify
* Full release build (thin LTO) takes ~8 min. For iteration use a non-LTO build in a separate target dir:
  `CARGO_TARGET_DIR=target/fast CARGO_PROFILE_RELEASE_LTO=false CARGO_PROFILE_RELEASE_CODEGEN_UNITS=16 cargo build --release`
* CI gates: `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`.
* Visual check: `tmux -L vchk new-session -d -s v -x 120 -y 34 ./target/fast/release/vanta; sleep 3; tmux -L vchk capture-pane -t v -p; tmux -L vchk kill-server` (check 200×50, 120×34, 100×30).
  * Switching pages saves `ui.startup_mode` to the user's real `~/.config/vanta/config.toml`. Prefer a throwaway copy: `mkdir -p /tmp/vxdg/vanta && cp ~/.config/vanta/config.toml /tmp/vxdg/vanta/` and pass `-e XDG_CONFIG_HOME=/tmp/vxdg` to `tmux new-session`. Use `capture-pane -e` to check colours (e.g. night dimming).
* Idle CPU: read `utime+stime` (fields 14+15) from `/proc/$(pgrep -nx vanta)/stat` twice, N seconds apart. Those are 1/100 s ticks, so % of a core = Δticks / N. `pgrep -f` matches the tmux server too, so use `-x`. As of v0.10.30 it's ~0.6–1% per page, plus ~0.8% for the cava child.

## 7. Ecosystem Boundaries
* **Vanta Core** (`vanta`): Contains the foundational monitors, UI framework, Extism host engine, and background threadpools.
* **Vanta Integrations** (`vanta-integrations` repo): A separate cargo workspace holding community-built WASM extensions. Each widget must be its own crate.

