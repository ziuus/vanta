"""Vanta Monitor TUI — screen-oriented entrypoint with theme support."""

from textual.app import App, ComposeResult
from textual.widgets import Header, Footer, Static
from textual.binding import Binding

from monitor.core.theme import DARK, LIGHT
from monitor.core.collectors import SystemCollector
from monitor.screens.overview import OverviewScreen
from monitor.screens.processes import ProcessesScreen
from monitor.screens.storage import StorageScreen
from monitor.screens.network import NetworkScreen
from monitor.screens.graphs import GraphScreen
from monitor.screens.widgets import WidgetsScreen
from monitor.screens.help_screen import HelpOverlay


class VantaMonitorTUI(App):
    """Keyboard-first system monitor with dedicated screen navigation."""

    CSS = f"""
    Screen {{
        background: {DARK['bg']};
    }}
    Screen.vanta-light {{
        background: {LIGHT['bg']};
    }}
    Header {{
        background: {DARK['bg']};
        border: none;
        color: {DARK['accent']};
    }}
    Header.vanta-light {{
        background: {LIGHT['bg']};
        color: {LIGHT['accent']};
    }}
    Footer {{
        background: {DARK['bg']};
        height: 1;
    }}
    Footer.vanta-light {{
        background: {LIGHT['bg']};
    }}
    #nav-bar {{
        height: 1;
        background: {DARK['surface']};
        border: solid {DARK['border']};
        padding: 0 1;
        dock: top;
    }}
    #nav-bar.vanta-light {{
        background: {LIGHT['surface']};
        border: solid {LIGHT['border']};
    }}
    #nav-bar Static {{
        color: {DARK['text_dim']};
    }}
    .vanta-light #nav-bar Static {{
        color: {LIGHT['text_dim']};
    }}
    """

    BINDINGS = [
        Binding("1", "switch('overview')", "Dashboard", show=True),
        Binding("2", "switch('processes')", "Processes", show=True),
        Binding("3", "switch('storage')", "Storage", show=True),
        Binding("4", "switch('network')", "Network", show=True),
        Binding("5", "switch('graphs')", "Graphs", show=True),
        Binding("6", "switch('widgets')", "Widgets", show=True),
        Binding("?", "help", "Help", show=True),
        Binding("q", "quit", "Quit", show=True),
        Binding("r", "refresh", "Refresh", show=False),
        Binding("T", "toggle_theme", "Theme", show=True),
    ]

    def __init__(self):
        super().__init__()
        self.collector = SystemCollector()
        self._theme = "light"
        self._screens = {
            "overview": OverviewScreen(),
            "processes": ProcessesScreen(),
            "storage": StorageScreen(),
            "network": NetworkScreen(),
            "graphs": GraphScreen(),
            "widgets": WidgetsScreen(),
        }

    def compose(self) -> ComposeResult:
        yield Header(show_clock=True)
        yield Static(id="nav-bar")
        yield Footer()

    def on_mount(self) -> None:
        self.title = "Vanta Monitor"
        # Install all screens so they can be referenced by name
        for name, screen in self._screens.items():
            self.install_screen(screen, name)
        self.push_screen("overview")
        self._apply_theme(self._theme)

    def _apply_theme(self, theme: str) -> None:
        self._theme = theme
        is_light = theme == "light"
        self.dark = not is_light

        hdr = self.query_one(Header)
        ftr = self.query_one(Footer)
        nav = self.query_one("#nav-bar", Static)
        for widget in (hdr, ftr, nav):
            if is_light:
                widget.add_class("vanta-light")
            else:
                widget.remove_class("vanta-light")

        for name, screen in self._screens.items():
            if is_light:
                screen.add_class("vanta-light")
            else:
                screen.remove_class("vanta-light")
            if hasattr(screen, "apply_theme"):
                screen.apply_theme(theme)

        acc = LIGHT if is_light else DARK
        nav.update(
            f"[1] [{acc['accent']}]Dashboard[/]  [2] Processes  [3] Storage  [4] Network  [5] Graphs  [6] Widgets  "
            f"[?] Help  [T] {theme}    [{acc['text_dim']}]q=quit[/]"
        )

    def action_switch(self, name: str) -> None:
        """Switch to a named screen via install_screen/push_screen."""
        if name not in self._screens:
            return
        # Push named screen — it's a no-op target if already on it
        self.push_screen(name)

    def action_help(self) -> None:
        help_screen = HelpOverlay(self._theme)
        self.push_screen(help_screen)

    def action_refresh(self) -> None:
        screen = self.screen
        if hasattr(screen, "_refresh"):
            screen._refresh()

    def action_toggle_theme(self) -> None:
        new = "dark" if self._theme == "light" else "light"
        self._apply_theme(new)

    def on_screen_resume(self, screen) -> None:
        """Re-apply theme classes when a screen is resumed (re-shown)."""
        is_light = self._theme == "light"
        if is_light:
            screen.add_class("vanta-light")
        else:
            screen.remove_class("vanta-light")
        if hasattr(screen, "apply_theme"):
            screen.apply_theme(self._theme)


def main():
    app = VantaMonitorTUI()
    app.run()


if __name__ == "__main__":
    main()
