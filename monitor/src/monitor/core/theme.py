"""Theme palettes and helpers for Vanta Monitor.

The TUI supports a small set of named presets. Screens use the palette returned
by ``get_palette(theme_name)`` for Rich markup colors and use
``is_light_theme(theme_name)`` to decide whether to add the ``vanta-light`` CSS
class for light-background layouts.
"""

from __future__ import annotations

from typing import Final

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
    "yellow": "#ca8a04",
    "red": "#dc2626",
    "error_bg": "rgba(220, 38, 38, 0.10)",
    "overlay_bg": "rgba(255, 255, 255, 0.90)",
    "overlay_surface": "#ffffff",
}

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
    "yellow": "#eab308",
    "red": "#ef4444",
    "error_bg": "rgba(239, 68, 68, 0.12)",
    "overlay_bg": "rgba(10, 10, 15, 0.88)",
    "overlay_surface": "#0f0f1a",
}

MONOKAI: Final[dict[str, str]] = {
    "bg": "#111216",
    "surface": "#1a1c23",
    "surface_alt": "#272b34",
    "border": "#3a3f4b",
    "text": "#f8f8f2",
    "text_muted": "#a6accd",
    "text_dim": "#7f849c",
    "accent": "#a6e22e",
    "green": "#a6e22e",
    "yellow": "#ffd866",
    "red": "#ff6188",
    "error_bg": "rgba(255, 97, 136, 0.16)",
    "overlay_bg": "rgba(10, 10, 15, 0.88)",
    "overlay_surface": "#1a1c23",
}

NORD_LIGHT: Final[dict[str, str]] = {
    "bg": "#eceff4",
    "surface": "#ffffff",
    "surface_alt": "#e5e9f0",
    "border": "#cfd6e4",
    "text": "#2e3440",
    "text_muted": "#4c566a",
    "text_dim": "#6b7280",
    "accent": "#5e81ac",
    "green": "#5e9b6b",
    "yellow": "#b48e3e",
    "red": "#bf616a",
    "error_bg": "rgba(191, 97, 106, 0.10)",
    "overlay_bg": "rgba(255, 255, 255, 0.90)",
    "overlay_surface": "#ffffff",
}

THEMES: Final[dict[str, dict[str, str]]] = {
    "light": LIGHT,
    "dark": DARK,
    "monokai": MONOKAI,
    "nord-light": NORD_LIGHT,
}

THEME_ORDER: Final[list[str]] = ["light", "dark", "monokai", "nord-light"]
LIGHT_THEMES: Final[set[str]] = {"light", "nord-light"}

# Map palette dict keys → CSS variable names
_CSS_VAR_MAP: Final[dict[str, str]] = {
    "bg": "bg",
    "surface": "surface",
    "surface_alt": "surface-alt",
    "border": "border",
    "text": "text",
    "text_muted": "text-muted",
    "text_dim": "text-dim",
    "accent": "accent",
    "green": "green",
    "yellow": "yellow",
    "red": "red",
    "error_bg": "error-bg",
    "overlay_bg": "overlay-bg",
    "overlay_surface": "overlay-surface",
}


def get_palette(name: str | None) -> dict[str, str]:
    if not name:
        return LIGHT
    return THEMES.get(name, LIGHT)


def is_light_theme(name: str | None) -> bool:
    if not name:
        return True
    return name in LIGHT_THEMES


def next_theme_name(current: str | None) -> str:
    current = current or THEME_ORDER[0]
    if current not in THEME_ORDER:
        return THEME_ORDER[0]
    idx = THEME_ORDER.index(current)
    return THEME_ORDER[(idx + 1) % len(THEME_ORDER)]


def theme_label(name: str | None) -> str:
    name = name or "light"
    return name.replace("-", " ")


def theme_to_css_vars(pal: dict[str, str]) -> dict[str, str]:
    """Convert a palette dict to {css_var_name: value} for the stylesheet."""
    return {
        f"${css_name}": pal[key]
        for key, css_name in _CSS_VAR_MAP.items()
        if key in pal
    }
