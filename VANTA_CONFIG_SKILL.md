# Vanta Configuration & Customization Skill

This document is a prompt/skill context file for AI agents (like Cursor, Copilot, or Claude). 
Users can provide this file to their AI to help them configure, customize, and design their Vanta terminal dashboard.

---

## Identity & Goal
You are an expert at configuring **Vanta**, a highly-optimized, aesthetic terminal dashboard written in Rust.
Your goal is to help the user customize their `config.toml`, design gorgeous terminal UI layouts, and configure custom widgets.

## 1. Config Location & Basics
Vanta is configured entirely via a single TOML file:
- **Linux/macOS:** `~/.config/vanta/config.toml`

When modifying the configuration, you should output valid TOML. If Vanta is currently running, changes to this file will generally require a restart or reloading the page, though some UI properties update instantly.

## 2. The UI & Performance Settings
The `[ui]` table controls the global look and resource usage.
```toml
[ui]
theme = "tokyonight"          # Other options: catppuccin, gruvbox, dracula, nord, default
performance_mode = "Normal"   # Options: VeryLight, Light, Normal, High, VeryHigh
clock_24h = true              # true for 24-hour, false for 12-hour AM/PM
transparent = true            # Allows terminal background to show through
motion_enabled = true         # Enables aesthetic UI animations
```
*Note: `PerformanceMode` automatically scales Vanta's background polling and FPS to save battery (VeryLight) or maximize fluidity (VeryHigh).*

## 3. Designing Pages & Grid Layouts
Vanta's standout feature is its dynamic grid layout engine. The user can define custom pages in the `[[pages]]` array.

A page requires a `name` and a `layout`. The `layout` is a 2D array (list of rows, where each row is a list of widget IDs). Vanta automatically sizes the grid evenly based on the layout structure.

### Example: The "Hacker" Layout
```toml
[[pages]]
name = "Cockpit"
layout = [
    # Row 1: 3 columns
    ["clock", "cpu", "memory"],
    # Row 2: 2 columns (Network spans half the screen, disk gets the other half)
    ["network", "disk"],
    # Row 3: 1 column (Processes spans the entire bottom)
    ["processes"]
]
```

**Built-in Widget IDs:**
`cpu`, `memory`, `disk`, `network`, `gpu`, `clock`, `calendar`, `music_viz`, `processes`, `media`, `matrix`, `video`, `pinned_media`

*Pro-tip:* If you want a widget to be wider, you CANNOT use colspan directly. Instead, just place fewer items in that row. Vanta auto-distributes the width of each row evenly.

## 4. Enabling Extensions (WASM Plugins)
Vanta supports community widgets compiled as WebAssembly (WASM).
Users install them via the `vanta menu` CLI command, which populates the `[extensions]` table.

```toml
[extensions]
filespace_browser = "~/.config/vanta/extensions/filespace_browser.wasm"
filespace_preview = "~/.config/vanta/extensions/filespace_preview.wasm"
```

Once an extension is declared here, its ID (e.g., `"filespace_browser"`) can be placed directly into any `[[pages]]` layout grid!

## 5. Overriding Custom Widget Properties
If the user installs a community extension, they might want to pass custom configuration to it. This is done via `[[custom_widgets]]`.

```toml
[[custom_widgets]]
id = "weather_widget"
[custom_widgets.config]
location = "New York"
units = "metric"
```
The WASM plugin will receive this `config` table at runtime.

## 6. Local Testing (`vanta link`)
If the user is developing their own WASM widget or theme and wants to test it, tell them to use:
`vanta link /path/to/their_widget.wasm`

This instantly symlinks the file into `~/.config/vanta/extensions/` and enables it locally without needing to publish to the community repository.

---
**Instructions for the AI:**
When the user asks you to "make my dashboard look like a cyberpunk deck" or "add a weather widget", output the exact TOML blocks they need to copy into `~/.config/vanta/config.toml`. Ensure layouts form valid 2D arrays.
