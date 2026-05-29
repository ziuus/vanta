# Vanta

A **super aesthetic** terminal dashboard — dark, sleek, absorbs everything into one cohesive TUI.
Now includes a built-in **system monitor** with TUI + web dashboard.

> Canonical project: **Vanta**. The old GlowMon monitor is being folded into `vanta/monitor/`.

```
┌─────────────────────────────────────────────────────────────┐
│ ⌘ VANTA  │  CPU:12%  RAM:3.5G/16G  DISK:45G/500G          │
├─────────────────────────────────────────────────────────────┤
│                    VANTA ASCII LOGO                          │
│              [ FOCUS ] [ WORK ] [ CHILL ] [ MONITOR ]       │
└─────────────────────────────────────────────────────────────┘
```

## How to Run

```bash
cd ~/Projects/vanta
./bin/vanta.sh
```

**That's it!** You'll see the interactive mode selector.

```
 ┌──────────────────────────────────────────────────────────┐
 │  VANTA - Terminal Dashboard                              │
 │  ───────────────────────────────                         │
 │                                                          │
 │  [1] FOCUS    → splash + clock                           │
 │         ~20-30 MB • Minimal, clean                       │
 │                                                          │
 │  [2] WORK     → clock + calendar + yazi                  │
 │         ~60-100 MB • Productive                          │
 │                                                          │
 │  [3] CHILL    → cava + cmatrix + quotes                  │
 │         ~40-80 MB • Aesthetic mode                       │
 │                                                          │
 │  [4] ALL      → Every module                             │
 │         ~120-250 MB • Full experience                    │
 │                                                          │
 │  [5] MONITOR  → System monitor TUI (Textual dashboard)   │
 │         CPU, GPU, Memory, Network, Processes             │
 │                                                          │
 └──────────────────────────────────────────────────────────┘
```

## Features

### Core (Toggle on Demand)
| Module | Description | RAM | Toggle |
|--------|-------------|-----|--------|
| **Monitor** | Built-in system monitor TUI (CPU/GPU/Mem/Disk/Net/Procs) | ~30-50 MB | `Ctrl+g` |

### Vanta Monitor (Built-in)
A custom Python system monitor with both **TUI** (Textual) and **Web** (Flask) interfaces:
- CPU usage per-core, frequency, sparkline history
- Memory usage with bar + sparkline
- GPU utilization, temperature, VRAM (via pynvml)
- Disk usage with bar chart
- Network upload/download speed
- Audio visualizer (simulated)
- Process manager (sort by CPU, kill/stop/resume)
- Calendar, clock, uptime, system temperature

### Standalone monitor app
The monitor is also self-contained inside `monitor/`, so it can be run and packaged independently without becoming a separate project:

```bash
cd ~/Projects/vanta/monitor
pip install -e .
vanta-monitor tui
vanta-monitor web
vanta-monitor both

# shorter aliases
vtui
vweb
vboth
```

### Toggleable Modules
| Module | Description | RAM | Toggle |
|--------|-------------|-----|--------|
| btop / jtop | System monitor (CPU/GPU/RAM/Disk/Net) | ~15-35 MB | `Ctrl+b` |
| tty-clock | Large font clock | ~1-5 MB | `Ctrl+k` |
| cal | Compact calendar | ~1-5 MB | `Ctrl+c` |
| cava | Music visualizer | ~5-15 MB | `Ctrl+v` |
| cmatrix | Matrix rain effect | ~5-10 MB | `Ctrl+m` |
| yazi | File manager | ~15-40 MB | `Ctrl+f` |

### System Modules
| Module | Description | RAM |
|--------|-------------|-----|
| fastfetch/neofetch | System info banner | ~5-10 MB |
| bandwhich/iftop | Bandwidth monitor | ~5-15 MB |
| df -h | Disk usage | ~1 MB |
| lazydocker | Container manager | ~10-30 MB |

### Aesthetic Modules
| Module | Description | RAM |
|--------|-------------|-----|
| pipes.sh | Falling pipes effect | ~5-10 MB |
| quotes | Random quotes + cowsay | ~1 MB |
| weather | wttr.in weather | ~1 MB |
| splash | Vanta ASCII logo | ~1 MB |
| custom text | Quotes, status, info | <1 MB |
| wallpaper | Static image (kitty) | ~10-50 MB |

### Utility Modules
| Module | Description | RAM |
|--------|-------------|-----|
| ncmpcpp | Music player | ~5-15 MB |
| lazygit | Git TUI | ~15-30 MB |
| notes | Quick notes panel | ~1 MB |
| monitor-web | Web dashboard on port 5000 | ~30-50 MB |

## Modes

| Mode | Modules | Est. RAM |
|------|---------|----------|
| **Focus** | splash + clock | ~20-30 MB |
| **Work** | splash + clock + calendar + yazi + lazygit | ~60-100 MB |
| **Chill** | splash + cava + cmatrix + quotes + pipes | ~40-80 MB |
| **Monitor** | System monitor TUI (Textual) | ~30-50 MB |

*Add btop anytime with `Ctrl+b` — ~15-35 MB extra*

## Quick Start

```bash
# Navigate to vanta
cd ~/Projects/vanta

# Make scripts executable
chmod +x bin/* modules/*

# Start dashboard (interactive mode selector)
./bin/vanta.sh

# Or start with specific mode directly
./bin/vanta.sh focus     # splash + clock (~20-30 MB)
./bin/vanta.sh work      # clock + calendar + yazi + lazygit (~60-100 MB)
./bin/vanta.sh chill     # cava + cmatrix + quotes + pipes (~40-80 MB)
./bin/vanta.sh all       # all modules (~120-250 MB)
./bin/vanta.sh monitor   # system monitor TUI

# Monitor with web dashboard
./bin/vanta.sh monitor --web   # Web on http://localhost:5000
./bin/vanta.sh monitor --both  # TUI + web simultaneously

# Toggle modules inside tmux
./bin/vanta-toggle clock
./bin/vanta-toggle monitor

# Switch modes
./bin/vanta-mode focus
./bin/vanta-mode monitor
```

**First time?** Just run:
```bash
cd ~/Projects/vanta
./bin/vanta.sh
```

You'll see the interactive mode selector — pick Focus/Work/Chill/All/Monitor.

## Keybinds

| Key | Action |
|-----|--------|
| `Ctrl+b` | Toggle btop (system monitor) |
| `Ctrl+v` | Toggle cava visualizer |
| `Ctrl+g` | Toggle monitor TUI |
| `Ctrl+k` | Toggle clock |
| `Ctrl+c` | Toggle calendar |
| `Ctrl+m` | Toggle cmatrix |
| `Ctrl+f` | Toggle yazi |
| `Ctrl+t` | Toggle custom text panel |
| `Ctrl+w` | Toggle wallpaper |
| `Ctrl+i` | Toggle sysinfo |
| `Ctrl+n` | Toggle network |
| `h/j/k/l` | Navigate panes |

### Monitor TUI Keybinds
| Key | Action |
|-----|--------|
| `j` | Move process selection down |
| `k` | Move process selection up |
| `x` | Kill selected process |
| `r` | Refresh |
| `q` | Quit |

## Theme

**Tokyo Night** (default)

### Install Tokyo Night for your tools:

**Kitty** (~/.config/kitty/kitty.conf):
```
background_opacity 0.95
foreground #c0caf5
color0 #1a1b26
color1 #f7768e
color2 #9ece6a
color3 #e0af68
color4 #7aa2f7
color5 #bb9af7
color6 #7dcfff
color7 #c0caf5
```

**btop** (save to ~/.config/btop/themes/tokyo-night.theme):
```
BTOP_THEME_NAME="Tokyo Night"
BTOP_COLOR_BG="#1a1b26"
BTOP_COLOR_MAIN="#7aa2f7"
BTOP_COLOR_SECOND="#bb9af7"
```

**Fonts**
- JetBrainsMono Nerd Font (recommended)
- FiraCode Nerd Font
- Cascadia Code Nerd Font

## Requirements

### Core (Required)
- tmux

### Recommended
- tty-clock, cal, cava, cmatrix, yazi
- fastfetch or neofetch
- lazygit
- fortune, cowsay

### Monitor Dependencies
- Python 3.10+
- `textual`, `psutil`, `flask`, `flask-cors` (auto-installed on first run)
- `pynvml` (optional, for GPU stats)

### Optional
- bandwhich (network)
- pipes.sh (aesthetic)
- ncmpcpp + mpd (music)
- kitty (images/wallpaper)
- lolcat (colored output)

## Installation

```bash
# Arch Linux
sudo pacman -S tmux tty-clock cal cava cmatrix yazi fastfetch lazygit fortune cowsay pipes-git

# Or with yay
yay -S pipes-git bandwhich

# Optional: music player
sudo pacman -S ncmpcpp mpd

# Monitor dependencies
pip install textual psutil flask flask-cors pynvml
```

## Project Structure

```
vanta/
├── README.md
├── bin/
│   ├── vanta.sh          # Main launcher
│   ├── vanta-select      # Interactive mode selector
│   ├── vanta-toggle      # Module toggle script
│   └── vanta-mode        # Mode switcher
├── config/
│   ├── vanta.tmux.conf   # tmux config
│   └── vanta.theme       # Color theme
├── monitor/              # Built-in system monitor (Python)
│   ├── config.json       # Monitor configuration
│   └── src/monitor/
│       ├── app.py         # Textual TUI
│       ├── server.py      # Flask web API + dashboard
│       ├── launcher.py    # TUI/Web launcher
│       └── dashboard.html # Web frontend
└── modules/              # Tmux launcher scripts
    ├── clock.sh
    ├── calendar.sh
    ├── monitor.sh         # System monitor launcher
    └── ...
```

## Memory Usage

**Total estimate:** ~120-300 MB depending on active modules

That's extremely efficient — lighter than a single Chrome tab.

---

*Vanta — absorb everything into one.*
