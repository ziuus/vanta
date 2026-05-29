#!/bin/bash
#
# matrix.sh - cmatrix for Vanta
#

# Check if cmatrix is installed
if ! command -v cmatrix &>/dev/null; then
    echo "Error: cmatrix not found. Install with: sudo pacman -S cmatrix"
    exit 1
fi

# Tokyo Night green theme
exec cmatrix -C green