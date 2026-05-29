#!/bin/bash
#
# weather.sh - Weather display for Vanta
#

if command -v wttr &>/dev/null; then
    exec wttr
elif command -v weather &>/dev/null; then
    exec weather
else
    # Check if curl is available
    if command -v curl &>/dev/null; then
        curl -s "wttr.in?format=3"
    else
        echo "Install curl or wttr"
    fi
fi