#!/bin/bash
#
# yazi.sh - Yazi file manager for Vanta
#

# Check if yazi is installed
if ! command -v yazi &>/dev/null; then
    echo "Error: yazi not found. Install with: cargo install yazi"
    exit 1
fi

# Initialize yazi
exec yazi