#!/bin/bash
#
# system-info.sh - Fastfetch system info for Vanta
#

if command -v fastfetch &>/dev/null; then
    exec fastfetch
elif command -v pfetch &>/dev/null; then
    exec pfetch
elif command -v neofetch &>/dev/null; then
    exec neofetch
else
    echo "Install fastfetch for best experience"
    echo "sudo pacman -S fastfetch"
    uname -a
fi