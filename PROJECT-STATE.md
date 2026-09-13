# Vanta Project State

## Completed Features
- Built a 4-tab aesthetic dashboard (Dashboard, Monitor, Aesthetic, Workspace)
- Switched default file manager and obsidian vault tracking to `~` (Home directory)
- Implemented **Dynamic Resizing**: Use `Ctrl+Left` and `Ctrl+Right` to resize currently focused column in Dashboard and Workspace modes.
- Implemented **File Manager Image Previews**: Images are parsed natively via `image` crate and rendered as ANSI half-blocks directly inside the terminal.
- Implemented **In-App Task/Agenda Editing**: Pressing `e` while focused on Tasks or Agenda temporarily suspends the TUI and opens the file in your system `$EDITOR` (e.g. `vim` or `nano`), seamlessly restoring the UI afterwards.
- Global deslop: Stripped bold formatting to ensure it doesn't look clunky; adjusted padding on widgets.

## Known Issues / User Feedback
- OS Logos (e.g. Arch, Ubuntu) use braille which sometimes renders poorly on certain terminal fonts. (Fallback to fastfetch-like ASCII may be needed).
- Font size constraints: Some text elements (like the clock) are inherently large because they use block ASCII art. The user can switch `clock_font` to something smaller in `~/.config/vanta/config.toml` if desired.

## Next Steps
- Revisit Calendar and Weather widgets if the user desires left-aligned instead of center-aligned text to minimize edge padding.
- Monitor NPM package usage (v0.4.0 recently pushed via CI).

## Major Epic: Dynamic Layout Engine
- **Goal**: Maximum customizability for the end user.
- **Features Planned**:
  - Migrate from hardcoded `src/screens` to a data-driven layout engine.
  - Allow users to define custom workspaces in `~/.config/vanta/config.toml`.
  - Allow users to map and position components (widgets) into custom grids/splits.
  - Support for adding external/custom components.
