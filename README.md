<div align="center">
  <h1>vanta</h1>
  <p><strong>Your machine, one pane.</strong><br/>
  A fast, aesthetic terminal system dashboard in Rust.</p>
  <a href="https://website-pi-seven-nty4ogjp2f.vercel.app"><img src="https://img.shields.io/badge/Landing_Page-030712?style=for-the-badge&logo=vercel&logoColor=white" alt="Landing Page" /></a>
  <a href="https://github.com/ziuus/vanta/releases"><img src="https://img.shields.io/github/v/release/ziuus/vanta?style=for-the-badge&color=4A9E8E" alt="Latest Release" /></a>
  <br /><br />
  <img src="docs/screenshot.png" alt="Vanta Dashboard" width="800" />
</div>

---

Vanta collapses everything you care about into one terminal pane: CPU, memory, disk, network, GPU, processes, now-playing, a block-digit clock, a calendar, and an audio visualizer. Keyboard only. ~3 MB binary, ~2–5% CPU at the default 30 fps.

## Pages

| Key | Page | What's on it |
|-----|------|--------------|
| `1` | **Dashboard** | System facts + distro logo, semicircle gauges, CPU graph & cores, storage, big clock, now playing, visualizer, top processes, status, memory, network, calendar |
| `2` | **Monitor** | btop-style: CPU / memory / disk I/O / network / GPU graphs on top, full process table below (sort, filter, tree, kill) |
| `3` | **Aesthetic** | Huge clock, calendar, matrix rain, spinning donut, full-width visualizer |
| `?` | **Help** | Keybind reference overlay |

## Keys

| Key | Action |
|-----|--------|
| `1` `2` `3` | Switch page (persisted as startup page) |
| `Tab` / `Shift-Tab` | Cycle panel focus · `Esc` clears |
| `T` | Next theme (persisted) |
| `v` | Visualizer style: bars / mirror / wave |
| `+` / `-` | Sample faster / slower |
| `Space` `n` `p` | Play/pause · next · previous (MPRIS, any page) |
| `<` `>` | Volume down / up |
| `q` | Quit |

**Processes (Monitor):** `↑↓ PgUp PgDn Home End` select · `/` filter (`Enter` keeps, `Esc` clears) · `s` sort field · `r` reverse · `t` tree · `←→` fold · `c` full command · `k` SIGTERM · `K` SIGKILL

**Calendar (focused):** `←→` month · `↑↓` year · `Home` today

## Themes

`dark` · `catppuccin` · `tokyo-night` · `nord` · `gruvbox` · `dracula` · `light` · `solarized-light`

## Configuration

`~/.config/vanta/config.toml` (respects `XDG_CONFIG_HOME`). Every key is optional.

```toml
[ui]
refresh_rate = 0.5      # seconds between samples
fps = 30                # render rate (animations)
theme = "dark"
startup_mode = "dashboard"

[widgets]               # all default to true
cpu = true
memory = true
disk = true
network = true
gpu = true
clock = true
calendar = true
music_viz = true
processes = true
media = true
matrix = true
video = true
```

## Install

Needs a Rust toolchain and `libdbus-1-dev` (for MPRIS). Optional runtime tools: `cava` (visualizer), `nmcli`, `docker`, `checkupdates` (status panel), `nvidia-smi`.

```bash
cargo install --git https://github.com/ziuus/vanta
# or
git clone https://github.com/ziuus/vanta && cd vanta && cargo run --release
```

## Architecture

All telemetry is collected on a single background sampler thread (`/proc`, `/sys`, sysinfo, D-Bus) into lock-guarded snapshots; the render loop only reads snapshots and never blocks on I/O.

| Layer | Technology |
|-------|------------|
| TUI | Ratatui 0.29 + Crossterm |
| Telemetry | `sysinfo`, `/proc`, `/sys/class/{hwmon,drm,power_supply}` |
| GPU | NVIDIA (`nvidia-smi`), AMD (sysfs), Intel (clock only) |
| Media | MPRIS over D-Bus (`dbus` crate) |
| Audio viz | `cava` raw output, idle wave fallback |

MIT · built by [zius](https://github.com/ziuus)
