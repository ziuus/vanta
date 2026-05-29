#!/bin/bash
#
# network.sh - Bandwidth monitor for Vanta
#

if command -v bandwhich &>/dev/null; then
    exec bandwhich
elif command -v iftop &>/dev/null; then
    exec iftop -i "$(ip route | awk '/default/ {print $5; exit}')"
elif command -v nload &>/dev/null; then
    exec nload
else
    echo "Install bandwhich for best experience"
    echo "sudo pacman -S bandwhich"
    # Fallback to simple speedtest
    command -v speedtest-cli &>/dev/null && exec speedtest-cli
    echo "No network monitor found"
fi