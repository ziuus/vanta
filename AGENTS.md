# vanta — agent notes

Rust TUI system dashboard (ratatui 0.29 + crossterm). Single binary, no runtime deps beyond libdbus.

## Build / verify
```bash
cargo build --release                       # ~12s incremental; binary at target/release/vanta
cargo fmt --all -- --check && cargo clippy --all-targets -- -D warnings && cargo test   # == CI
```

## Visual verification (no GUI needed)
```bash
tmux new-session -d -s vanta -x 200 -y 50 "./target/release/vanta"; sleep 3
tmux send-keys -t vanta 1        # 1/2/3 = pages
tmux capture-pane -t vanta -p    # add -e for ANSI colours
tmux kill-session -t vanta
```
Check 100×34 (minimum) and 200×50. `top -p $(pgrep -x vanta)` should show ~3–5% CPU.

## Architecture rules
- `src/monitors/*`: `sample()` runs on the sampler thread (`monitors/mod.rs::start`); `render()` only reads the module's snapshot. Do slow work *before* taking the lock.
- Never do I/O, spawn processes, or decode images inside a `render()`.
- `screens/*` = pages; `screens::panel()` = shared border/title chrome; `screens::render_panel()` = zoom.
- Strings: `widgets::meter::ellipsize` (char-safe). Colours: `theme.usage()/temp()`.
- Config lives at `~/.config/vanta/config.toml`; every field has a serde default.
