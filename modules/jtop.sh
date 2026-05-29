#!/bin/bash
#
# jtop.sh - Beautiful system monitor (btop fork focused on processes)
# Shows top CPU and top memory users
#

if ! command -v jtop &>/dev/null; then
    echo "jtop not found. Install with:"
    echo "  pip install jtop"
    echo ""
    echo "Alternative: btop (sudo pacman -S btop)"
    exit 1
fi

exec jtop