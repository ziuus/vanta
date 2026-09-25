<div align="center">

# vanta

**Your machine, one pane.**

A fast, aesthetic terminal system dashboard in Rust. The most complete keyboard-driven terminal dashboard — one pane, four modes, zero mouse.

[![npm](https://img.shields.io/npm/v/@ziuus/vanta?style=for-the-badge&logo=npm&logoColor=white&color=CB3837)](https://www.npmjs.com/package/@ziuus/vanta)
[![CI](https://img.shields.io/github/actions/workflow/status/ziuus/vanta/ci.yml?style=for-the-badge&label=CI)](https://github.com/ziuus/vanta/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/ziuus/vanta?style=for-the-badge&color=4A9E8E)](https://github.com/ziuus/vanta/releases)
[![License](https://img.shields.io/badge/License-MIT-blue?style=for-the-badge)](LICENSE)
[![Landing page](https://img.shields.io/badge/Landing_Page-030712?style=for-the-badge&logo=vercel&logoColor=white)](https://website-pi-seven-nty4ogjp2f.vercel.app)

<br />

<img src="docs/dashboard.png" alt="vanta dashboard page" width="900" />

</div>

---

Vanta collapses everything you care about into one terminal pane: CPU, memory, disk,
network, GPU (and radial thermals), processes, now-playing, GitHub contributions, multiple timezones, a calendar, and an audio
visualizer. Keyboard only. **~3 MB binary, ~2–5% CPU** at the default 30 fps.

```bash
npm install -g @ziuus/vanta && vanta
```

## Install

<table>
<tr><th align="left">Method</th><th align="left">Command</th><th align="left">Notes</th></tr>
<tr>
  <td><strong>npm</strong></td>
  <td><code>npm install -g @ziuus/vanta</code></td>
  <td>Fetches the prebuilt <code>linux-x64</code> binary. No Rust needed.</td>
</tr>
<tr>
  <td><strong>Prebuilt binary</strong></td>
  <td><a href="https://github.com/ziuus/vanta/releases/latest">Releases</a></td>
  <td>Download the <code>tar.gz</code>, drop <code>vanta</code> on your <code>PATH</code>.</td>
</tr>
<tr>
  <td><strong>cargo</strong></td>
  <td><code>cargo install --git https://github.com/ziuus/vanta</code></td>
  <td>Any architecture. Needs a Rust toolchain.</td>
</tr>
<tr>
  <td><strong>From source</strong></td>
  <td><code>git clone https://github.com/ziuus/vanta &amp;&amp; cd vanta &amp;&amp; cargo run --release</code></td>
  <td>For hacking on it.</td>
</tr>
</table>

**One-liner, no install:**

```bash
npx @ziuus/vanta@latest
```

### Requirements

Linux (vanta reads `/proc` and `/sys`). Building from source also needs `libdbus`:

```bash
sudo pacman -S dbus              # Arch
sudo apt install libdbus-1-dev   # Debian / Ubuntu
sudo dnf install dbus-devel      # Fedora
```

Optional tools that light up extra panels — vanta degrades quietly without them:

| Tool | Unlocks |
|------|---------|
| `cava` | Audio visualizer (falls back to an idle wave) |
| `nmcli` | Wi-Fi SSID + signal strength |
| `nvidia-smi` | NVIDIA GPU utilisation, VRAM, temp |
| `docker` | Running-container count |
| `checkupdates` | Pending Arch package updates |

## Managing vanta

```bash
vanta --version                 # what am I running
vanta --help                    # usage + keys

npm update -g @ziuus/vanta      # update  (npm)
npm uninstall -g @ziuus/vanta   # remove  (npm)

cargo install --git https://github.com/ziuus/vanta --force   # update  (cargo)
cargo uninstall vanta                                        # remove  (cargo)

rm -rf ~/.config/vanta          # drop config + persisted theme/page
```

Where things live:

| Path | What |
|------|------|
| `~/.config/vanta/config.toml` | Config, plus the theme / page / visualizer vanta persists for you |
| `$(npm root -g)/@ziuus/vanta/bin/` | The npm-installed binary |
| `~/.cargo/bin/vanta` | The cargo-installed binary |

Vanta respects `XDG_CONFIG_HOME` if you set it.

## Pages

| Key | Page | What's on it |
|-----|------|--------------|
| `1` | **Dashboard** | System facts + distro logo, semicircle gauges, CPU graph & cores, storage, big clock, now playing, visualizer, top processes, interactive status, memory, network, calendar |
| `2` | **Monitor** | btop-style: CPU / memory / disk I/O / network / GPU graphs on top, full process table below (sort, filter, tree, kill) |
| `3` | **Aesthetic** | Huge clock, calendar, matrix rain, spinning donut, pinned media (photos), full-width visualizer |
| `4` | **Workspace** | Built-in File Manager (with image previews), Agenda, Tasks, RSS feeds, and Obsidian vault viewer |
| `?` | **Help** | Keybind reference overlay |

### `2` — Monitor

Every graph on top, every process below. Sort it, filter it, tree it, kill it.

<img src="docs/monitor.png" alt="vanta monitor page" width="900" />

### `3` — Aesthetic

For when the build is running and you want something to look at. Includes support for displaying a custom pinned image.

**Includes 7 immersive ambient scenes:**
- Audio Visualizer
- Matrix Rain
- Flip Clock
- Topographic Map
- True 3D Starfield (reacts to CPU spikes and music bass)
- Conway's Game of Life (auto-seeding)
- Terminal Snowfall (piles up and melts dynamically)

<img src="docs/aesthetic.png" alt="vanta aesthetic page" width="900" />

### `4` — Workspace

Turn your terminal into a productivity hub with a built-in file manager (with image previews), RSS news reader, tasks, agenda, and Obsidian vault viewer (with markdown syntax highlighting). Press `e` on any focused panel to edit the content in your `$EDITOR`.


### `?` — Help

Every key, on screen, on any page.

<img src="docs/help.png" alt="vanta help overlay" width="900" />

## Keys

| Key | Action |
|-----|--------|
| `1` `2` `3` `4` | Switch page (persisted as startup page) |
| `Tab` / `Shift-Tab` | Cycle panel focus · `Esc` clears |
| `Enter` | Zoom the focused panel to the full page · `Esc` back |
| `T` | Next theme (persisted) |
| `v` | Visualizer style: bars / mirror / wave / peaks |
| `+` / `-` | Sample faster / slower |
| `Space` `n` `p` | Play/pause · next · previous (MPRIS, any page) |
| `<` `>` | Volume down / up |
| `?` | Help overlay |
| `q` | Quit |
| `~` / `F12` | Toggle Debug Logs overlay (`s` saves them to file) |
| `S` / `,` | Open Settings overlay |
| `Ctrl + ←/→` | Resize dashboard columns horizontally (when focused) |
| `Ctrl + ↑/↓` | Resize dashboard components vertically (when focused) |

**Processes (Monitor):** `↑↓ PgUp PgDn Home End` select · `/` filter (`Enter` keeps, `Esc` clears) · `s` sort field · `r` reverse · `t` tree · `←→` fold · `c` full command · `k` SIGTERM · `K` SIGKILL (press twice to confirm, `x` cancels)

**Workspace:** `e` opens the active note, agenda, task list, or file in your `$EDITOR`.

**Calendar (focused):** `←→` month · `↑↓` year · `Home` today

## Themes

`dark` · `catppuccin` · `tokyo-night` · `nord` · `gruvbox` · `dracula` · `light` · `solarized-light`

Cycle with `T` — your choice is written back to the config file.

### Custom Themes

You can easily load custom themes dynamically without recompiling Vanta.

1. Create a `themes` folder inside your config directory (e.g. `~/.config/vanta/themes/`).
2. Drop in a `.toml` file with your theme name (e.g., `cyberpunk.toml`).
3. Define your colors using hex codes:
   ```toml
   bg = "#000000"
   accent = "#00ff00"
   secondary = "#ff00ff"
   surface = "#111111"
   text = "#ffffff"
   dim = "#555555"
   green = "#00ff00"
   yellow = "#ffff00"
   red = "#ff0000"
   ```
4. Press `T` in Vanta to cycle to it, or set it explicitly in your `config.toml` (`theme = "cyberpunk"`).

## 🔌 Extensions & Plugins

- **Configuration:** Run `vanta config` to open the interactive settings editor.
- **First-time Setup:** The same setup menu runs automatically when starting Vanta for the first time.


Vanta is fully extensible via WebAssembly (WASM). You can install community widgets, or build your own!

- **Install via Menu:** Run `vanta menu` to browse and install extensions interactively.
- **Local Dev:** Use `vanta link /path/to/my_widget.wasm` to instantly test your custom plugin locally without publishing it.

Read more about building plugins at the [vanta-integrations](https://github.com/ziuus/vanta-integrations) repository.

## Configuration & Customization

Vanta is highly configurable via `~/.config/vanta/config.toml`. You can:
- Change the layout, refresh rates, and themes
- Add your own **Custom Widgets** using shell commands or files
- Install community-built **WASM Extensions** to add entirely new UI panels without recompiling

For a complete guide to all configuration options, extensions, and custom widgets, please read:
👉 **[docs/CONFIGURATION.md](docs/CONFIGURATION.md)**

## Architecture

All telemetry is collected on a single background sampler thread (`/proc`, `/sys`,
sysinfo, D-Bus) into lock-guarded snapshots; the render loop only reads snapshots
and never blocks on I/O.

| Layer | Technology |
|-------|------------|
| TUI | Ratatui 0.29 + Crossterm |
| Telemetry | `sysinfo`, `/proc`, `/sys/class/{hwmon,drm,power_supply}` |
| GPU | NVIDIA (`nvidia-smi`), AMD (sysfs), Intel (clock only) |
| Media | MPRIS over D-Bus (`dbus` crate) |
| Audio viz | `cava` raw output, idle wave fallback |

## Contributing

Issues and PRs welcome — bug reports, new themes, new panels, or a GPU vendor
that isn't covered yet.

```bash
git clone https://github.com/ziuus/vanta && cd vanta
cargo run --release

# CI runs exactly this — make it pass first
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test
```

Read **[CONTRIBUTING.md](CONTRIBUTING.md)** first — it covers how the code is laid
out, the sampler-thread rule (*never do I/O inside a `render()`*), the panel
degradation contract, and how to add a theme.

Good first contributions:

- A new theme in `src/theme.rs` (add a constructor + its name to `THEME_NAMES`)
- Per-interface network stats
- NVMe / extra `hwmon` temperature sources
- Mouse click-to-focus

## License

[MIT](LICENSE) — do what you like, keep the notice.

<div align="center">
<br />
built by <a href="https://github.com/ziuus">zius</a> · if it's useful, a ⭐ helps
</div>
