# Vanta – Contribution Guide

## Getting Started

```bash
git clone https://github.com/ziuus/vanta
cd vanta
cargo run
```

Vanta requires Rust 1.96+.

## Codebase

```
src/
├── main.rs          — entrypoint, terminal init, event loop
├── app.rs           — Theme, App state, screen routing, render
├── config.rs        — Config loading (TOML)
├── screens/
│   ├── mod.rs       — Screen enum
│   ├── overview.rs  — system monitoring view
│   └── widgets.rs   — flex/eye-candy view
├── monitors/
│   ├── mod.rs
│   ├── cpu.rs       — CPU bars + per-core display
│   └── memory.rs    — RAM + swap gauges
└── widgets/
    ├── mod.rs
    ├── clock.rs     — date/time display
    └── matrix.rs    — matrix rain animation
```

## Controls

| Key | Action |
|-----|--------|
| `1` | Overview (monitoring) |
| `2` | Widgets (eye candy) |
| `t` | Toggle theme |
| `q` | Quit |

## Adding a new widget

1. Create `src/widgets/<name>.rs` with `pub fn render(f: &mut Frame, area: Rect, theme: &app::Theme)`
2. Add `pub mod <name>;` to `src/widgets/mod.rs`
3. Render it from `src/screens/widgets.rs`

## Stack

- **Ratatui** 0.29 — TUI framework
- **Crossterm** 0.28 — terminal backend
- **sysinfo** 0.33 — system information
- **chrono** — date/time
- **rand** — random (matrix rain)

## Code Style

- `cargo fmt` before committing
- `cargo clippy` — no warnings
- Keep widget render functions stateless where possible
