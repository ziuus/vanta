"""Help overlay showing all keyboard bindings for the Vanta Monitor TUI."""

from textual.app import ComposeResult
from textual.screen import Screen
from textual.widgets import Static
from textual.containers import Horizontal, Vertical
from textual.binding import Binding

from monitor.core.theme import DARK, LIGHT


def _tag(color: str, text: str) -> str:
    return f"[{color}]{text}[/]"


KEYBINDS = [
    (
        "Navigation",
        [
            ("1", "Dashboard (everything)"),
            ("2", "File manager"),
            ("q", "Quit Vanta Monitor"),
        ],
    ),
    (
        "Dashboard controls",
        [
            ("W", "Cycle widget (clock/cal/matrix/viz)"),
        ],
    ),
    (
        "File manager (screen 2)",
        [
            ("j/k", "Move up/down"),
            ("l", "Enter directory"),
            ("h", "Parent directory"),
            ("~", "Go home"),
            ("g/G", "Top / bottom"),
        ],
    ),
    (
        "General",
        [
            ("?", "Toggle this help screen"),
            ("r", "Force refresh"),
            ("T", "Toggle light/dark theme"),
        ],
    ),
]


class HelpOverlay(Screen):
    """Modal help screen that overlays the current screen."""

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
        return LIGHT if self._theme_name == "light" else DARK

    def compose(self) -> ComposeResult:
        p = self.pal
        with Vertical(id="help-modal"):
            yield Static(f"{_tag(p['accent'], '◈ Vanta Dashboard — Keybinds')}", id="help-title")
            for category, binds in KEYBINDS:
                yield Static(f"{_tag(p['text_muted'], category)}", classes="help-category")
                for key, action in binds:
                    yield Horizontal(
                        Static(f"[{p['text']}]{key:<8}[/]", classes="help-key"),
                        Static(f"{_tag(p['text_dim'], action)}", classes="help-desc"),
                        classes="help-row",
                    )
            yield Static(
                f"{_tag(p['text_dim'], 'Press ')}{_tag(p['text'], 'Esc')}, "
                f"{_tag(p['text'], '?')}, or {_tag(p['text'], 'q')} "
                f"{_tag(p['text_dim'], 'to close')}",
                id="help-footer",
            )

    def apply_theme(self, theme: str) -> None:
        self._theme_name = theme
        is_light = theme == "light"
        if is_light:
            self.add_class("vanta-light")
        else:
            self.remove_class("vanta-light")

    CSS = """
    HelpOverlay {
        align: center middle;
        background: rgba(10, 10, 15, 0.88);
    }
    #help-modal {
        width: 54;
        height: auto;
        border: solid #06b6d4;
        background: #0f0f1a;
        padding: 1 2;
    }
    #help-title {
        text-style: bold;
        padding: 0 0 1 0;
        text-align: center;
    }
    .help-category {
        text-style: bold;
        padding: 1 0 0 0;
        border-top: solid #1e1e3f;
    }
    .help-row {
        height: 1;
        padding: 0 1;
    }
    .help-key {
        width: 10;
    }
    .help-desc {
        width: 1fr;
    }
    #help-footer {
        text-align: center;
        padding: 1 0 0 0;
    }
    
    .vanta-light HelpOverlay {
        background: rgba(255, 255, 255, 0.88);
    }
    .vanta-light #help-modal {
        border: solid #0891b2;
        background: #ffffff;
    }
    .vanta-light .help-category {
        border-top: solid #d1d5db;
    }
    """
