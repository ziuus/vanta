#!/bin/bash
#
# notes.sh - Quick notes panel for Vanta
#

NOTES_FILE="${VANTA_DIR:-~/Projects/vanta}/notes.txt"

if [ -f "$NOTES_FILE" ]; then
    cat "$NOTES_FILE"
else
    echo "No notes yet."
    echo "Create $NOTES_FILE to display notes here."
fi