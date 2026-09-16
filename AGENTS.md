# Vanta Architecture & Agent Guide

This document is intended for AI agents and developers working on the Vanta codebase. It outlines the architectural boundaries, rendering pipelines, and extension mechanisms.

## 1. Core Philosophy
Vanta is a highly-optimized, aesthetic terminal dashboard written in Rust using `ratatui`. 
* **Performance first**: The main render loop must never block on I/O. All data fetching (monitors, feeds) happens in background threads that update a shared state (`Summary` or `App` states).
* **Terminal limits**: UI components must gracefully handle terminal resizing and tiny viewports.
* **No bloat**: Core features should be universally useful. Niche features belong in extensions.

## 2. Rendering Pipeline
* **`src/main.rs`**: The entry point. Initializes the terminal, loads the config, boots background workers, and drives the TUI event loop (tick rate vs frame rate).
* **`src/app.rs`**: Holds the global state (`App`). Contains the navigation (`DashboardMode`) and handles all keystrokes.
* **`src/screens/dashboard.rs`**: The primary layout engine. It takes the `[[dashboard.layout]]` grid from the config and dynamically maps string IDs (e.g. `"cpu"`, `"cve_feed"`) to their rendering functions.

## 3. Configuration (`src/config.rs`)
Vanta is heavily driven by `config.toml`. 
* We use `serde` with `#[serde(default)]` to ensure backwards compatibility. 
* **Custom Pages**: Users can define `[[pages]]` with custom layouts. These are dynamically loaded into the top navigation bar at runtime.
* **Extensions**: The `[extensions]` table controls which compiled extensions are booted.

## 4. Extension Architecture (V1)
Vanta supports compile-time extensions. The core API is in `src/extension/mod.rs`.
* **`Extension` trait**: Bundles pages and components. Registered in `main.rs`.
* **`Page` trait**: Creates full-screen, dedicated views that are injected into the top-level `DashboardMode` navigation.
* **`Component` trait**: Creates isolated UI widgets. These widgets export an `id()` (e.g., `"cve_feed"`). If a user puts this string in their `layout` grid in `config.toml`, `dashboard.rs` will automatically query the `ExtensionManager` and render the component natively.
* *Note: Extensions must be explicitly enabled via `enabled = ["ext_name"]` in `config.toml`, otherwise they remain completely dormant in memory.*

## 5. Ecosystem Boundaries
* **Vanta Core** (`vanta`): Contains the foundational monitors, UI framework, and the `Extension` traits. It is exposed as both a binary and a library (`lib.rs`).
* **Vanta Integrations** (`vanta-integrations` repo): A separate cargo workspace holding community-built extensions. Agents asked to build new integrations should generally create a new crate in the integrations repository, depend on `vanta` as a library, and instruct the user to register the crate in their local `main.rs`.
