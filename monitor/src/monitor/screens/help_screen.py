"""Help overlay showing global and screen-specific keyboard bindings."""

from __future__ import annotations

from pathlib import Path

from textual.app import ComposeResult
from textual.binding import Binding
from textual.containers import Horizontal, Vertical
from textual.screen import Screen
from textual.widgets import Static

from monitor.core.theme import get_palette, is_light_theme, theme_label

STYLES_DIR = Path(__file__).resolve().parent.parent / "styles"


def _tag(color: str, text: str) -> str:
    return f"[{color}]{text}[/]"


KEYBINDS = [
    (
        "Navigation",
        [
            ("1", "Overview dashboard"),
            ("2", "Graphs / trends"),
            ("3", "File manager"),
            ("q", "Quit Vanta Monitor"),
        ],
    ),
    (
        "Overview",
        [
            ("j/k", "Move process selection"),
            ("/", "Search by name or PID"),
            ("c / C", "Cycle sort forward / backward"),
            ("u / U", "Toggle kernel / current-user filter"),
            ("F8", "Toggle flat/tree process view"),
            ("d", "Open process detail"),
            ("K", "Open signal menu"),
        ],
    ),
    (
        "General",
        [
            ("?", "Open or close help"),
            ("r", "Force refresh current screen"),
            ("T", "Toggle light/dark"),
            ("P", "Cycle theme preset"),
        ],
    ),
]


class HelpOverlay(Screen):
    BINDINGS = [
        Binding("escape", "dismiss", "Close"),
        Binding("?", "dismiss", "Close"),
        Binding("q", "dismiss", "Close"),
    ]

    def __init__(self, theme: str = "light"):
        super().__init__()
        self._theme_name = theme

    @property
    def pal(self) -> dict[str, str]:
        return get_palette(self._theme_name)

    def compose(self) -> ComposeResult:
        p = self.pal
        with Vertical(id="help-modal"):
            yield Static(f"{_tag(p['accent'], '◈ Vanta Monitor — Keybinds')}  [{p['text_dim']}]{theme_label(self._theme_name)}[/]", id="help-title")
            for category, binds in KEYBINDS:
                yield Static(_tag(p["text_muted"], category), classes="help-category")
                for key, action in binds:
                    yield Horizontal(
                        Static(f"[{p['text']}]{key:<8}[/]", classes="help-key"),
                        Static(_tag(p["text_dim"], action), classes="help-desc"),
                        classes="help-row",
                    )
            yield Static(
                f"{_tag(p['text_dim'], 'Press ')}{_tag(p['text'], 'Esc')}, {_tag(p['text'], '?')}, or {_tag(p['text'], 'q')} {_tag(p['text_dim'], 'to close')}",
                id="help-footer",
            )

    def apply_theme(self, theme: str) -> None:
        self._theme_name = theme
        if is_light_theme(theme):
            self.add_class("vanta-light")
        else:
            self.remove_class("vanta-light")

    CSS_PATH = str(STYLES_DIR / "help.tcss")
