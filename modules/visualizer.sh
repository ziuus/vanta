#!/bin/bash
#
# visualizer.sh - Cava music visualizer for Vanta
#

# Check if cava is installed
if ! command -v cava &>/dev/null; then
    echo "Error: cava not found. Install with: sudo pacman -S cava"
    exit 1
fi

# Check if there's audio playing
# Optional: only start if audio is detected
if command -v pacmd &>/dev/null; then
    if ! pacmd list-sinks | grep -q "running"; then
        echo "No audio detected. Starting cava anyway..."
    fi
fi

exec cava