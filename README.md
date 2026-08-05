<div align="center">
  <br />
  <h1>vanta</h1>
  <p>
    <strong>Your machine, one pane.</strong><br/>
    A blazingly fast, highly aesthetic terminal system dashboard built in Rust.
  </p>
  <a href="https://website-pi-seven-nty4ogjp2f.vercel.app">
    <img src="https://img.shields.io/badge/Live_Landing_Page-030712?style=for-the-badge&logo=vercel&logoColor=white" alt="Live Landing Page" />
  </a>
  <a href="https://github.com/ziuus/vanta/releases">
    <img src="https://img.shields.io/github/v/release/ziuus/vanta?style=for-the-badge&color=4A9E8E" alt="Latest Release" />
  </a>
  <a href="https://crates.io/crates/vanta">
    <img src="https://img.shields.io/crates/v/vanta?style=for-the-badge&color=28c840" alt="Crates.io" />
  </a>
  <br /><br />
  <img src="docs/screenshot.png" alt="Vanta Dashboard Screenshot" width="800" />
</div>

<hr />

## 🚀 The Ultimate Terminal Cockpit

**Vanta** collapses all the system metrics you care about into a single, beautiful terminal pane. It's fully keyboard-driven with zero mouse dependency and no floating tabs.

With three focused pages plus an on-demand help overlay, Vanta transforms from a dense technical monitor to pure terminal eye-candy with a single keystroke.

## ⚡ Pages

| Key | Page | Description |
|-----|------|-------------|
| `1` | **Dashboard** | All-in-one: CPU, Memory, Disk, Network, GPU, Clock, Calendar, Media Player, System Info, live Processes, and the visualizer. |
| `2` | **Monitor** | btop-style detail: large CPU/Mem/Net/Disk/GPU graphs on top, full interactive process table below (sort, search, tree, kill). |
| `3` | **Aesthetic** | Pure eye candy: Matrix Rain, Calendar, Visualizer, Clock, and a rotating 3D donut demo. |
| `?` | **Help** | Floating overlay: keybinds, active theme, and config path. Toggle over any page. |

---

## 🎹 Keyboard Mastery

No mouse. No touch. Just keys.

### Global Actions
| Key | Action |
|-----|--------|
| `1`–`3` | Switch pages (Dashboard / Monitor / Aesthetic) |
| `?` | Toggle help & settings overlay |
| `T` | Cycle themes (changes persist to config) |
| `v` | Cycle visualizer style (bars / mirror / wave) |
| `Tab` / `Shift‑Tab` | Cycle panel focus |
| `↑` `↓` `←` `→` | Navigate the active panel |
| `Esc` | Clear panel focus / close overlay |
| `q` | Quit |

### Process Explorer (Monitor page)
| Key | Action |
|-----|--------|
| `s` | Cycle sort fields (PID, CPU, Mem, Name, RSS) |
| `/` | Enter search mode |
| `t` | Toggle tree view |
| `←` `→` | Collapse / Expand tree node |
| `i` | View process details |
| `c` | Toggle compact command view |
| `k` | Send Kill signal (SIGTERM) |

### Media Player (Dashboard page)
| Key | Action |
|-----|--------|
| `Space` | Play / Pause |
| `n` / `p` | Next / Previous track |
| `+` / `-` | Volume Up / Down |

---

## 🎨 Professional Themes

Cycle through four meticulously designed color palettes using `T`:

1. **Dark** (Default) — Deep, immersive terminal blacks.
2. **Light** — High-contrast, clean paper white.
3. **Dracula** — Classic purple-accented dark theme.
4. **Solarized Light** — Warm, easy-on-the-eyes daylight theme.

Your selection is automatically persisted to `~/.config/vanta/config.toml`.

---

## ⚙️ Configuration

Vanta is fully modular. Configure widgets, refresh rates, and your default theme via `~/.config/vanta/config.toml`:

```toml
[ui]
refresh_rate = 0.5
theme = "dark"
# Optional: render any image as ASCII art in the profile widget.
# Leave unset to use the built-in vanta logo. Supports ~ expansion.
image_path = "~/Pictures/avatar.png"

[widgets]
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
cmatrix = true
```

---

## 📥 Installation

You will need the Rust toolchain installed.

**From Cargo:**
```bash
cargo install --git https://github.com/ziuus/vanta
```

**From Source:**
```bash
git clone https://github.com/ziuus/vanta.git
cd vanta
cargo run --release            # Live hardware monitoring
```

---

## 🏗️ Architecture

| Layer | Technology |
|-------|------------|
| **Core Framework** | Rust + Ratatui `0.29` |
| **Terminal IO** | Crossterm |
| **Telemetry** | `sysinfo`, `/proc`, `/sys` (NVIDIA/AMD/Intel GPU) |
| **Media Sync** | MPRIS (DBus / `playerctl`) |
| **Audio Viz** | `cava` + PulseAudio |
| **Time** | `chrono` |

---

<div align="center">
  <p>Built with 🖤 by <a href="https://github.com/ziuus">zius</a></p>
  <p>Released under the MIT License.</p>
</div>