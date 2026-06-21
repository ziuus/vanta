"""Vanta Monitor TUI app shell.

The shell owns screen registration, global keybindings, and theme preset state.
Each screen renders its own header/footer because Textual screens replace the
active surface; the app stays intentionally thin.
"""

from __future__ import annotations

from pathlib import Path

from textual.app import App
from textual.binding import Binding
from textual.events import Key

from monitor.core.dashboard_config import load_dashboard_config
from monitor.core.theme import (
    get_palette,
    is_light_theme,
    next_theme_name,
    theme_to_css_vars,
)
from monitor.screens.filemanager import FileManagerScreen
from monitor.screens.graphs import GraphsScreen
from monitor.screens.help_screen import HelpOverlay
from monitor.screens.overview import OverviewScreen
from monitor.screens.widgets_screen import WidgetsScreen

CONFIG_PATH = Path(__file__).resolve().parents[2] / "config.json"
STYLES_DIR = Path(__file__).resolve().parent / "styles"


class VantaMonitorTUI(App[None]):
    """Keyboard-first system monitor with overview, graphs, and file views."""

    CSS_PATH = str(STYLES_DIR / "base.tcss")

    BINDINGS = [
        Binding("1", "switch('overview')", "Overview", show=True),
        Binding("2", "switch('graphs')", "Graphs", show=True),
        Binding("3", "switch('files')", "Files", show=True),
        Binding("4", "switch('widgets')", "Widgets", show=True),
        Binding("?", "help", "Help", show=True),
        Binding("q", "quit", "Quit", show=True),
        Binding("r", "refresh", "Refresh", show=False),
    ]

    def __init__(self) -> None:
        # Must set _theme before super().__init__() because get_css_variables()
        # is called during the superclass constructor.
        config = load_dashboard_config(CONFIG_PATH)
        self._theme = config.ui.theme or "light"
        super().__init__()
        self._screens = {
            "overview": OverviewScreen(),
            "graphs": GraphsScreen(),
            "files": FileManagerScreen(),
            "widgets": WidgetsScreen(),
        }

    def get_css_variables(self) -> dict[str, str]:
        """Provide CSS variables for the current theme palette."""
        # Get Textual's built-in variables first ($background, $foreground, etc.)
        base_vars = super().get_css_variables()
        # Merge our custom theme palette variables on top
        base_vars.update(theme_to_css_vars(get_palette(self._theme)))
        return base_vars

    def on_mount(self) -> None:
        for name, screen in self._screens.items():
            self.install_screen(screen, name)
        self.push_screen("overview")
        self._apply_theme(self._theme)

    def _apply_theme(self, theme: str) -> None:
        self._theme = theme
        self.dark = theme not in ("light", "nord-light")
        # Update CSS variables via the stylesheet
        self.refresh_css()
        # Apply .vanta-light class on the app root for structural overrides
        if is_light_theme(theme):
            self.add_class("vanta-light")
        else:
            self.remove_class("vanta-light")
        for screen in self._screens.values():
            if hasattr(screen, "apply_theme"):
                screen.apply_theme(theme)

    def action_switch(self, name: str) -> None:
        if name not in self._screens:
            return
        self.switch_screen(name)
        screen = self.screen
        if hasattr(screen, "apply_theme"):
            screen.apply_theme(self._theme)

    def action_help(self) -> None:
        self.push_screen(HelpOverlay(self._theme))

    def action_refresh(self) -> None:
        screen = self.screen
        if hasattr(screen, "_refresh"):
            screen._refresh()

    def action_toggle_theme(self) -> None:
        self._apply_theme("dark" if self._theme in ("light", "nord-light") else "light")

    def action_cycle_theme_preset(self) -> None:
        self._apply_theme(next_theme_name(self._theme))

    def on_screen_resume(self, screen) -> None:
        if hasattr(screen, "apply_theme"):
            screen.apply_theme(self._theme)

    def on_key(self, event: Key) -> None:
        key = (event.key or "").lower()
        if key in {"t", "shift+t"}:
            self.action_toggle_theme()
            event.stop()
        elif key in {"p", "shift+p"}:
            self.action_cycle_theme_preset()
            event.stop()


def main() -> None:
    app = VantaMonitorTUI()
    app.run()


if __name__ == "__main__":
    main()
