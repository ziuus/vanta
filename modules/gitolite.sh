#!/bin/bash
#
# gitolite.sh - LazyGit for Vanta
#

if ! command -v lazgit &>/dev/null; then
    if ! command -v lazygit &>/dev/null; then
        echo "Error: lazygit not found"
        echo "Install: go install github.com/jesseduffield/lazygit@latest"
        exit 1
    fi
    exec lazygit
fi

exec lazgit