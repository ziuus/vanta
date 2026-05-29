#!/bin/bash
# Vanta Monitor Module — launch the system monitor TUI and/or web dashboard
# Usage: vanta-toggle monitor [tui|web|both]

VANTA_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
MONITOR_DIR="$VANTA_DIR/monitor"
LAUNCHER="$MONITOR_DIR/src/monitor/launcher.py"

# Check if running in tmux
if [ -z "$TMUX" ]; then
    echo "Vanta Monitor requires a tmux session."
    echo "Start with: tmux new-session -s vanta"
    exit 1
fi

# Check if deps installed
if ! python3 -c "import textual" 2>/dev/null; then
    echo "Installing dependencies (textual, psutil, flask)..."
    pip install textual psutil flask flask-cors pynvml 2>/dev/null || \
        pip install --user textual psutil flask flask-cors pynvml 2>/dev/null
fi

MODE="${1:-tui}"

if [ "$MODE" = "tui" ]; then
    exec python3 "$LAUNCHER" tui
elif [ "$MODE" = "web" ]; then
    exec python3 "$LAUNCHER" web
elif [ "$MODE" = "both" ]; then
    exec python3 "$LAUNCHER" both
fi
