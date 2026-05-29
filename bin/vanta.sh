#!/bin/bash
#
# vanta.sh - Main Vanta launcher
# Usage: vanta.sh [mode] [--web]
# Modes: focus, work, chill, all, monitor
#
# Vanta is a modular tmux-based dashboard system
# https://github.com/ziuus/vanta
#

VANTA_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TMUX_CONF="$VANTA_DIR/config/vanta.tmux.conf"

# Ensure tmux
if ! command -v tmux &>/dev/null; then
    echo "Error: tmux not found. Install with: sudo pacman -S tmux"
    exit 1
fi

show_help() {
    cat <<'EOF'
╔════════════════════════════════════════════════════╗
║                  V A N T A                          ║
║         Modular tmux Dashboard System               ║
╚════════════════════════════════════════════════════╝

USAGE:  vanta.sh [mode] [--web]

MODES:
  focus      Minimal: splash + clock (~20-30 MB)
  work       Full: clock + calendar + yazi + lazygit (~60-100 MB)
  chill      Visual: cava + cmatrix + pipes + quotes (~40-80 MB)
  all        Everything (~120-250 MB)
  monitor    System monitor TUI (Textual dashboard)
  monitor --web   Web dashboard on http://localhost:5000
  monitor --both  TUI + web simultaneously

KEYBINDS (within tmux):
  Ctrl+b    Toggle btop
  Ctrl+v    Toggle cava (audio viz)
  Ctrl+k    Toggle clock (tty-clock)
  Ctrl+c    Toggle calendar
  Ctrl+m    Toggle cmatrix
  Ctrl+f    Toggle yazi (file manager)
  Ctrl+g    Toggle system monitor TUI
  Ctrl+i    Toggle sysinfo
  Ctrl+n    Toggle network monitor
  Ctrl+d    Toggle disk usage
  Ctrl+x    Toggle docker/container view
  Alt+m     Toggle ncmpcpp (music)
  Alt+g     Toggle lazygit
  Ctrl+w    Close pane
  Ctrl+q    Close window

EXAMPLES:
  vanta.sh                    Start in focus mode
  vanta.sh work               Start in work mode
  vanta.sh monitor            Start system monitor TUI
  vanta.sh monitor --both     TUI + web dashboard
EOF
}

start_monitor() {
    local web_flag=""
    if [ "$1" = "--web" ]; then
        web_flag="web"
    elif [ "$1" = "--both" ]; then
        web_flag="both"
    else
        web_flag="tui"
    fi

    # Kill existing session first
    tmux kill-session -t vanta 2>/dev/null

    echo "Starting Vanta Monitor ($web_flag mode)..."
    echo ""

    # Create a fresh session with just the monitor
    tmux -f "$TMUX_CONF" new-session -d -s vanta -n monitor \
        "$VANTA_DIR/modules/monitor.sh $web_flag"

    # Web display: add a pane showing the URL
    if [ "$web_flag" = "both" ] || [ "$web_flag" = "web" ]; then
        sleep 2
        tmux split-window -h -t vanta 'watch -n 2 "curl -s http://localhost:5000/api/stats | python3 -m json.tool 2>/dev/null || echo Loading..."'
    fi

    tmux attach-session -t vanta
}

main() {
    local mode="${1:-focus}"
    local extra="${2:-}"

    case "$mode" in
        monitor)
            start_monitor "$extra"
            ;;
        focus|work|chill|all)
            exec "$VANTA_DIR/bin/vanta-select" "$mode"
            ;;
        help|--help|-h)
            show_help
            ;;
        *)
            echo "Unknown mode: $mode"
            show_help
            exit 1
            ;;
    esac
}

main "$@"
