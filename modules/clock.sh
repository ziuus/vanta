#!/bin/bash
#
# clock.sh - Clock module for Vanta
#

CMD="tty-clock -c -C 2"

# Check if tty-clock is installed
if ! command -v tty-clock &>/dev/null; then
    echo "Error: tty-clock not found. Install with: sudo pacman -S tty-clock"
    exit 1
fi

exec $CMD