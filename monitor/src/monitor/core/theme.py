"""Theme palettes for Vanta Monitor TUI.

Each screen generates its CSS using these constants via f-strings.
Toggle the `vanta-light` CSS class on the Screen widget to switch themes.
"""

from __future__ import annotations

from typing import Final

DARK: Final[dict[str, str]] = {
    "bg": "#0a0a0f",
    "surface": "#0f0f1a",
    "surface_alt": "#1a1a2e",
    "border": "#1e1e3f",
    "text": "#cbd5e1",
    "text_muted": "#64748b",
    "text_dim": "#4a5568",
    "accent": "#06b6d4",
    "green": "#22c55e",
    "red": "#ef4444",
    "error_bg": "rgba(239, 68, 68, 0.12)",
}

LIGHT: Final[dict[str, str]] = {
    "bg": "#f5f5f0",
    "surface": "#ffffff",
    "surface_alt": "#e8ecf0",
    "border": "#d1d5db",
    "text": "#1a1a1a",
    "text_muted": "#6b7280",
    "text_dim": "#9ca3af",
    "accent": "#0891b2",
    "green": "#16a34a",
    "red": "#dc2626",
    "error_bg": "rgba(220, 38, 38, 0.10)",
}
