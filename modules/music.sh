#!/bin/bash
#
# music.sh - Ncmpcpp music player for Vanta
#

if ! command -v ncmpcpp &>/dev/null; then
    echo "Error: ncmpcpp not found"
    echo "Install: sudo pacman -S ncmpcpp"
    exit 1
fi

# Check if mpd is running
if ! pgrep -x mpd > /dev/null; then
    echo "MPD not running. Starting mpd..."
    mpd &
    sleep 1
fi

exec ncmpcpp