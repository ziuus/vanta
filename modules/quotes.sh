#!/bin/bash
#
# quotes.sh - Quotes/ASCII art for Vanta
#

# Custom quotes array
QUOTES=(
    "The only way to do great work is to love what you do."
    "Stay hungry, stay foolish."
    "Code is like humor. When you have to explain it, it's bad."
    "First, solve the problem. Then, write the code."
    "Simplicity is the ultimate sophistication."
    "Talk is cheap. Show me the code."
    "Any fool can write code that a computer can understand. Good programmers write code that humans can understand."
    "Premature optimization is the root of all evil."
    "The best error message is the one that never shows up."
    "Perfection is achieved not when there is nothing more to add, but when there is nothing left to take away."
)

# Pick random quote
INDEX=$((RANDOM % ${#QUOTES[@]}))
QUOTE="${QUOTES[$INDEX]}"

# Display with cowsay if available
if command -v cowsay &>/dev/null; then
    echo "$QUOTE" | cowsay -f ghostbusters
elif command -v figlet &>/dev/null; then
    echo "$QUOTE" | figlet -f small
else
    echo "$QUOTE"
fi