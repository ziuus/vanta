#!/bin/bash
#
# splash.sh - Vanta ASCII splash screen
#

if command -v lolcat &>/dev/null; then
    CAT="| lolcat"
else
    CAT=""
fi

# Vanta ASCII art
cat <<'EOF' $CAT
 ██████╗██╗   ██╗██████╗ ███████╗██████╗     ██████╗ ███████╗ █████╗ ██████╗
██╔════╝╚██╗ ██╔╝██╔══██╗██╔════╝██╔══██╗    ██╔══██╗██╔════╝██╔══██╗██╔══██╗
██║      ╚████╔╝ ██████╔╝█████╗  ██████╔╝    ██║  ██║█████╗  ███████║██████╔╝
██║       ╚██╔╝  ██╔══██╗██╔══╝  ██╔══██╗    ██║  ██║██╔══╝  ██╔══██║██╔══██╗
╚██████╗   ██║   ██████╔╝███████╗██║  ██║    ██████╔╝███████╗██║  ██║██║  ██║
 ╚═════╝   ╚═╝   ╚═════╝ ╚══════╝╚═╝  ╚═╝    ╚═════╝ ╚══════╝╚═╝  ╚═╝╚═╝  ╚═╝

    [ F O C U S ] [ W O R K ] [ C H I L L ]
EOF

echo ""
echo "Press Ctrl+b prefix for tmux commands"
echo "Toggle modules: Ctrl+v (cava) | Ctrl+m (matrix) | Ctrl+f (yazi)"
echo ""
date +"%A, %B %d %Y | %H:%M:%S"