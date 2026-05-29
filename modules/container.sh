#!/bin/bash
#
# container.sh - Docker/LXC container manager for Vanta
#

if command -v lazydocker &>/dev/null; then
    exec lazydocker
elif command -v docker &>/dev/null; then
    if [ "$1" = "ps" ]; then
        docker ps -a
    elif [ "$1" = "images" ]; then
        docker images
    else
        docker ps -a
    fi
elif command -v lxc &>/dev/null; then
    exec lxc list
else
    echo "No container runtime found (docker/lxc)"
fi