#!/bin/bash
#
# calendar.sh - Calendar module for Vanta
#

# Check if cal is available (usually pre-installed)
if ! command -v cal &>/dev/null; then
    echo "Error: cal not found"
    exit 1
fi

# Optional: use ncal for horizontal layout
if command -v ncal &>/dev/null; then
    exec cal -3
else
    exec cal -3
fi