"""Config-driven widgets screen: clock, calendar, matrix, music viz, fastfetch."""

from __future__ import annotations

from pathlib import Path

from textual.app import ComposeResult
from textual.binding import Binding
from textual.events import Resize
from textual.reactive import reactive
from textual.screen import Screen
from textual.widgets import Footer, Header, Static

from monitor.core.dashboard_config import load_dashboard_config
from monitor.core.dashboard_widgets import WidgetRenderCache
from monitor.core.theme import get_palette, is_light_theme

CONFIG_PATH = Path(__file__).resolve().parents[3] / "config.json"
STYLES_DIR = Path(__file__).resolve().parent.parent / "styles"


class WidgetsScreen(Screen):
    """Config-driven widgets grid: clock, calendar, matrix, music viz, fastfetch."""

    BINDINGS = [
        Binding("escape", "app.switch('overview')", "Back", show=True),
    ]

    _refresh_tick = reactive(0)

    def __init__(self) -> None:
        super().__init__()
        self.dashboard_config = load_dashboard_config(CONFIG_PATH)
        self._theme_name = self.dashboard_config.ui.theme or "light"
        self._cache = WidgetRenderCache()
        self._dom_ready = False
        self._refresh_timer = None

    CSS_PATH = str(STYLES_DIR / "widgets.tcss")

    @property
    def pal(self) -> dict[str, str]:
        return get_palette(self._theme_name)

    def _is_enabled(self, name: str) -> bool:
        return self.dashboard_config.widget(name).enabled

    def _settings(self, name: str) -> dict:
        return self.dashboard_config.widget(name).settings

    def compose(self) -> ComposeResult:
        yield Header(show_clock=True)
        yield Static(id="widgets-body")
        yield Footer()

    def on_mount(self) -> None:
        self._dom_ready = True
        self._refresh_tick = 0
        self._rebuild()
        self._refresh_timer = self.set_interval(1.0, self._tick)

    def on_resize(self, _: Resize) -> None:
        self._reflow()

    def _tick(self) -> None:
        self._refresh_tick += 1
        self._refresh()

    def _reflow(self) -> None:
        height = self.size.height
        compact = self.dashboard_config.compact_mode_for_height(height)
        tiny = self.dashboard_config.ultra_compact_mode_for_height(height)
        self.set_class(compact and not tiny, "compact")
        self.set_class(tiny, "tiny")

    def _rebuild(self) -> None:
        """Full rebuild — only called on mount and theme change."""
        self._cache = WidgetRenderCache()  # flush cache
        self._refresh()

    def _refresh(self) -> None:
        """Re-render all enabled widgets with fresh cached content."""
        body = self.query_one("#widgets-body", Static)
        if not body:
            return

        p = self.pal
        blocks: list[str] = []

        # ── Clock ──
        if self._is_enabled("clock"):
            clock_text = self._cache.render("clock", self._settings("clock"))
            blocks.append(f"[{p['accent']}]{'⏰ Clock':^}[/]")
            blocks.append(f"[{p['text']}]{clock_text}[/]")

        # ── Calendar ──
        if self._is_enabled("calendar"):
            cal_text = self._cache.render("calendar", self._settings("calendar"))
            blocks.append("")
            blocks.append(f"[{p['accent']}]{'📅 Calendar':^}[/]")
            blocks.append(f"[{p['text']}]{cal_text}[/]")

        # ── Matrix ──
        if self._is_enabled("matrix"):
            matrix_text = self._cache.render("matrix", self._settings("matrix"))
            blocks.append("")
            blocks.append(f"[{p['accent']}]{'🌧️ Matrix':^}[/]")
            blocks.append(f"[{p['green']}]{matrix_text}[/]")

        # ── Music viz ──
        if self._is_enabled("music_viz"):
            music_text = self._cache.render("music_viz", self._settings("music_viz"))
            blocks.append("")
            blocks.append(f"[{p['accent']}]{'🎵 Music Viz':^}[/]")
            blocks.append(f"[{p['accent']}]{music_text}[/]")

        # ── Fastfetch ──
        if self._is_enabled("fastfetch"):
            ff_text = self._cache.render("fastfetch", self._settings("fastfetch"))
            blocks.append("")
            blocks.append(f"[{p['accent']}]{'💻 System':^}[/]")
            blocks.append(f"[{p['text']}]{ff_text}[/]")

        body.update("\n".join(blocks))

    def apply_theme(self, theme: str) -> None:
        self._theme_name = theme
        if is_light_theme(theme):
            self.add_class("vanta-light")
        else:
            self.remove_class("vanta-light")
        if self._dom_ready:
            self._rebuild()
