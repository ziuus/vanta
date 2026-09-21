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

## 5. Ecosystem Boundaries
* **Vanta Core** (`vanta`): Contains the foundational monitors, UI framework, Extism host engine, and background threadpools.
* **Vanta Integrations** (`vanta-integrations` repo): A separate cargo workspace holding community-built WASM extensions. Each widget must be its own crate.

