"""Help overlay showing all keyboard bindings for the Vanta Monitor TUI."""

from textual.app import ComposeResult
from textual.screen import Screen
from textual.widgets import Static
from textual.containers import Horizontal, Vertical
from textual.binding import Binding


KEYBINDS = [
    (
        "Screen navigation",
        [
            ("1", "Dashboard"),
            ("2", "Processes"),
            ("3", "Storage"),
            ("4", "Network"),
            ("5", "Graphs"),
            ("q", "Quit Vanta Monitor"),
        ],
    ),
    (
        "Dashboard widget controls",
        [
            ("[", "Previous widget page"),
            ("]", "Next widget page"),
        ],
    ),
    (
        "Process actions (on Processes screen)",
        [
            ("k", "Kill selected process"),
            ("s", "Stop (suspend) selected process"),
            ("r", "Resume selected process"),
            ("t", "Cycle sort column"),
            ("Ctrl+T", "Toggle sort direction"),
            ("/", "Focus filter input"),
        ],
    ),
    (
        "General",
        [
            ("?", "Toggle this help screen"),
            ("r", "Force refresh current screen"),
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

    def compose(self) -> ComposeResult:
        with Vertical(id="help-modal"):
            yield Static("[#06b6d4]◈ Vanta Dashboard — Keybinds[/]", id="help-title")
            for category, binds in KEYBINDS:
                yield Static(f"[#64748b]{category}[/]", classes="help-category")
                for key, action in binds:
                    yield Horizontal(
                        Static(f"[#cbd5e1]{key:<8}[/]", classes="help-key"),
                        Static(f"[#94a3b8]{action}[/]", classes="help-desc"),
                        classes="help-row",
                    )
            yield Static("[#4a5568]Press [#cbd5e1]Esc[/], [#cbd5e1]?[/], or [#cbd5e1]q[/] to close[/]", id="help-footer")

    def apply_theme(self, theme: str) -> None:
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
