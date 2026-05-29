#!/bin/bash
#
# pipes.sh - Falling pipes effect for Vanta
#

if command -v pipes &>/dev/null; then
    exec pipes -p 2000 -f 100
elif [ -x /usr/bin/pipes.sh ]; then
    exec /usr/bin/pipes.sh
else
    # Inline pipes effect
    echo "Installing pipes..."
    echo "sudo pacman -S pipes.sh"
fi