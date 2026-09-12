# vanta — working status & handoff

> Living doc. Any agent continuing this work should read this first, then
> `CONTRIBUTING.md`. Keep it updated: what's done, in-flight, and next.
> Last updated: 2026-09-12.

## What vanta is

Aesthetic Rust TUI system dashboard (ratatui 0.29 + crossterm). CPU/mem/disk/
net/GPU/processes/now-playing/clock/visualizer/donut in one terminal pane.
Published as `@ziuus/vanta` (npm shim → downloads the linux-x64 binary from the
GitHub release). Maintainer: zius. Branch: `main` (pushes bypass branch
protection — the maintainer has admin bypass).

## Build / verify loop (do this for every change)

```bash
cargo fmt --all -- --check      # CI gate 1
cargo clippy --all-targets -- -D warnings   # CI gate 2
cargo test                      # CI gate 3
```
Then visual check in a real terminal via tmux (script -qec gives a 0×0 pty —
don't use it):
```bash
tmux kill-session -t vv 2>/dev/null
tmux new-session -d -s vv -x 175 -y 44   # user's size; also test 100x34 and 200x50
tmux send-keys -t vv './target/release/vanta' Enter
sleep 3
tmux capture-pane -t vv -p
tmux kill-session -t vv 2>/dev/null
```
Keys to exercise styles: `T` theme, `v` visualizer, `g` gauge style, `G` graph
style, `1/2/3` pages, `?` help.

## Hard constraints (do not violate)

- **Shell is fish.** Bare globs and `--include=*.rs` fail (`no matches found`).
  Wrap globby snippets in `sh -c '...'` or drop the glob.
- **`src/custom/source.rs` must never use `sh -c`** — argv split only. A config
  file is not a shell trust boundary. Don't "simplify" it into a shell call.
- **`docs/dashboard.png` deliberately blurs the Wi-Fi SSID and LAN address** —
  the repo is public. Preserve that redaction.
- **Never tell the user to `git pull`** — we commit directly in their working
  tree (same clone).
- **Don't push/publish unless asked.** (User has now said to push each stable
  change — see below.)
- **npm publishing is the user's job** ("I will deal the npm publishing").
- Commit trailer: `Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>`.
- Asynkor MCP tools are referenced in global CLAUDE.md but **not available** in
  this environment — proceed uncoordinated, don't nag.

## Conventions worth knowing

- **Per-widget style switching pattern:** a module-global `AtomicUsize` + a
  `STYLE_COUNT` const + `set_style(&str)` / `cycle_style()` / `style_name()`,
  read inside `render()`. Set from config at startup (`app.rs::App::new`) and
  cycled by a key (persist via `config.save()` + a toast). Used by
  `music_viz` (visualizer), `gauge`, `block_graph`. This is the backbone the
  future **settings menu** will drive.
- **Colour blending:** `theme::blend(a, b, t)` (truecolor lerp, snaps for
  non-RGB). `theme.usage(pct)` = thresholded green/yellow/red badge;
  `theme.usage_ramp(t)` = continuous gradient counterpart. Never hardcode
  colours — pull from the theme.
- Panels must degrade, not overflow (check `area.width/height`). Truncate with
  `meter::ellipsize` (char-safe), never byte slicing.
- Config: `~/.config/vanta/config.toml`. `[ui]` + `[widgets]` are runtime-mutable
  and saved surgically; `[[custom_widgets]]` is user-authored, never rewritten.
  New `UiConfig` fields need a default in `UiConfig::default()` AND serde
  `#[serde(default)]` (already on the struct) so old configs still load.

## Recently done (committed + pushed to main)

- `3ef514e` Reap timed-out custom widget commands (`kill()` then `wait()`).
- `e9cff3a` Redraw distro logos with triangle glyphs + gradient-fill graphs;
  added `theme::blend`. **NOTE: the logo redraw is NOT accepted — see TODO.**
- `0da36f1` Shade gauge arcs along the sweep (accent→yellow→red).
- `654d3ef` Shade the donut by luminance (dim→accent→text) so it reads as a lit
  3D surface instead of a flat silhouette.
- `b947da0` Ease the visualizer between frames (rise-fast/fall-slow) for a
  "liquid" cava-like flow.
- `50bf785` **Configurable gauge + graph styles.** gauges: arc | bars |
  vertical. graphs: block | braille. `ui.gauge_style` (default arc) +
  `ui.graph_style` (default block); `g`/`G` cycle at runtime + persist; help
  updated. **Verified on foot:** bars are clean btop-style bars; braille graphs
  render dense/solid (⢸⣿⣿⡇⣿⣿⣿) — the btop look the user wanted. Defaults stay on
  the old look; press `g`/`G` to switch. Braille is a strong candidate for the
  new default if the user confirms it looks good on their setup.

- `50bf785` **Configurable gauge + graph styles.** gauges: arc | bars |
  vertical. graphs: block | braille. `ui.gauge_style` (default arc) +
  `ui.graph_style` (default block); `g`/`G` cycle at runtime + persist; help
  updated. **Verified on foot:** bars are clean btop-style bars; braille graphs
  render dense/solid (⢸⣿⣿⡇⣿⣿⣿) — the btop look the user wanted. Defaults stay on
  the old look; press `g`/`G` to switch.
- `1825ec1` **Arch logo in braille.** Added a bitmap→braille packer
  (`braille_art`) + a procedural Arch mountain (`arch_bitmap`); renders smooth
  and solid on foot. `VANTA_LOGO=<id>` overrides the detected distro to preview
  any logo. Only Arch converted so far.

## In flight

_(nothing uncommitted right now)_

## TODO — requested, not yet started

Ordered by value to the user (who runs Arch, dracula/dark, foot, 175×44):

1. **Box-alignment audit** (highest value — the "clean and structured" ask).
   "Many boxes have free unaligned spaces on the sides and inside," reads as
   unstructured. Systematic pass over every panel's inner padding, column
   alignment, and wasted whitespace. Compare against btop's tight columns.
   Touch: `src/screens/*`, per-monitor `render()`s, `src/monitors/*`. Look for
   panels that under-fill their area (e.g. the SYSTEM facts are centred with big
   left gaps; status/memory columns don't align).
2. **Top bar** (`src/app.rs`, the `vanta ● cpu 30% · mem … · up 4h` header):
   cleaner, smaller, cooler. Currently one wide dotted line.
3. **Status box polish** (WIFI/IP/PKGS/LOAD/PROCS/BAT/TEMPS panel): alignment +
   visual tightening; align the label column and values.
4. **Settings menu** — in-app modal (like `help.rs`) to configure theme, gauge
   style, graph style, visualizer style, fps, refresh, widget on/off, clock.
   Backend already exists (the style atomics + `config.save()`); this is the UI.
   Bind to a key (`S` or `,`). List + arrow nav + enter/←→ to change. Worth
   confirming the UX with the user before building.
5. **Other distro logos → braille** (finish what `1825ec1` started). Author a
   bitmap per distro (ubuntu/fedora/debian/nixos/gentoo/opensuse + fallback) and
   verify each with `VANTA_LOGO=<id> ./target/release/vanta`. Circle-family
   (ubuntu/fedora/suse) can be procedural; debian swirl / nixos lambda are
   harder — hand bitmaps. Keep block art as the fallback for anything unverified.

## Reference: btop is the design target

User points at btop as "clean and structured." Traits to emulate: consistent
gradient meter bars everywhere, tight column alignment, corner labels, braille
graphs, no wasted interior whitespace. Their terminal is `foot` (renders braille
well), theme usually dracula or catppuccin or dark, 30fps.

## Known-but-untouched (user's call, low priority)

- `assets/image.png` / `assets/logo.png` tracked but unreferenced.
- README landing badge points at a Vercel preview domain.
- `~/.local/bin/vtui` dead pip console script; `.journey-*` orphan file.
