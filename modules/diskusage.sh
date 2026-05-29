#!/bin/bash
#
# diskusage.sh - Disk usage monitor for Vanta
#

# Show disk usage with colors
df -h | grep -v "tmpfs\|devtmpfs\|loop\|snap" | head -10

# Optional: show inodes
echo ""
echo "=== Inodes ==="
df -i | grep -v "tmpfs\|devtmpfs\|loop\|snap" | head -5