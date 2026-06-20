"""Full-screen widget gallery — clock, calendar, matrix, music viz, and more."""

from __future__ import annotations

import random
from datetime import datetime

from textual.app import ComposeResult
from textual.containers import Horizontal, Vertical
from textual.screen import Screen
from textual.widgets import Footer, Header, Static

from monitor.core.collectors import SystemCollector
from monitor.core.dashboard_config import load_dashboard_config
from monitor.core.dashboard_widgets import (
    MATRIX_CHARS,
    build_calendar_widget,
    build_custom_text_widget,
    build_pstree_widget,
    build_yazi_widget,
    WidgetRenderCache,
)
from monitor.core.theme import DARK, LIGHT
from pathlib import Path

CONFIG_PATH = Path(__file__).resolve().parents[3] / "config.json"


def _tag(color: str, text: str) -> str:
    return f"[{color}]{text}[/]"


class WidgetsScreen(Screen):
    """Dedicated full-screen view for all extra widgets."""

    def __init__(self):
        super().__init__()
        self.collector = SystemCollector()
        self.dashboard_config = load_dashboard_config(CONFIG_PATH)
        self._cache = WidgetRenderCache()
        self._tick = 0
        self._matrix_seed = random.randint(0, 2**31)
        self._refresh_timer = None
        self._theme_name = "light"

    def compose(self) -> ComposeResult:
        yield Header(show_clock=True)
        with Vertical(id="wg-body"):
            with Horizontal(id="wg-top-row", classes="wg-row"):
                yield Static(id="wg-clock", classes="wg-panel wg-clock-panel")
                yield Static(id="wg-calendar", classes="wg-panel wg-cal-panel")
            with Horizontal(id="wg-mid-row", classes="wg-row"):
                yield Static(id="wg-matrix", classes="wg-panel wg-matrix-panel")
                yield Static(id="wg-music", classes="wg-panel wg-music-panel")
            with Horizontal(id="wg-bot-row", classes="wg-row"):
                yield Static(id="wg-pstree", classes="wg-panel wg-info-panel")
                yield Static(id="wg-custom", classes="wg-panel wg-info-panel")
                yield Static(id="wg-yazi", classes="wg-panel wg-info-panel")
        yield Footer()

    def on_mount(self) -> None:
        self._refresh_timer = self.set_interval(1.0, self._refresh)
        self._refresh()

    @property
    def pal(self) -> dict[str, str]:
        return LIGHT if self._theme_name == "light" else DARK

    def _refresh(self) -> None:
        self.dashboard_config = load_dashboard_config(CONFIG_PATH)
        self._tick += 1
        p = self.pal

        # --- Clock (big, prominent) ---
        clock_cfg = self.dashboard_config.widget("clock").settings
        dt = datetime.now()
        time_part = dt.strftime(
            "%H:%M:%S" if clock_cfg.get("format", "24h") == "24h" else "%I:%M:%S %p"
        )
        date_part = dt.strftime("%A, %B %d, %Y")
        self.query_one("#wg-clock", Static).update(
            f"[bold]{_tag(p['text'], time_part)}[/]\n{_tag(p['text_muted'], date_part)}"
        )

        # --- Calendar ---
        cal_cfg = self.dashboard_config.widget("calendar").settings
        self.query_one("#wg-calendar", Static).update(build_calendar_widget(cal_cfg))

        # --- Matrix ---
        matrix_cfg = self.dashboard_config.widget("matrix").settings
        self.query_one("#wg-matrix", Static).update(self._render_matrix(matrix_cfg))

        # --- Music viz ---
        music_cfg = self.dashboard_config.widget("music_viz").settings
        self.query_one("#wg-music", Static).update(self._render_music(music_cfg))

        # --- Pstree ---
        pstree_cfg = self.dashboard_config.widget("pstree").settings
        pstree_text = self._cache.render("pstree", pstree_cfg)
        self.query_one("#wg-pstree", Static).update(
            f"{_tag(p['text_muted'], 'Process Tree')}\n{pstree_text}"
        )

        # --- Custom text ---
        custom_cfg = self.dashboard_config.widget("custom_text").settings
        custom_text = build_custom_text_widget(custom_cfg)
        self.query_one("#wg-custom", Static).update(custom_text)

        # --- Yazi ---
        yazi_cfg = self.dashboard_config.widget("yazi").settings
        yazi_text = build_yazi_widget(yazi_cfg) if yazi_cfg.get("enabled", True) else "yazi disabled"
        self.query_one("#wg-yazi", Static).update(
            f"{_tag(p['text_muted'], '└─ Files (yazi)')}\n{yazi_text}"
        )

    def _render_matrix(self, cfg: dict) -> str:
        """Animated matrix — dark green gradient on dark, deep teal on light."""
        width = int(cfg.get("width", 24))
        height = int(cfg.get("height", 8))
        density = max(0.1, float(cfg.get("density", 1.0)))
        rng = random.Random(self._matrix_seed + self._tick)
        p = self.pal

        if self._theme_name == "light":
            bright, mid, dim = p["green"], "#6b9e6b", "#c8e6c9"
        else:
            bright, mid, dim = "#00ff41", "#00cc33", "#005500"

        lines: list[str] = []
        for _ in range(height):
            chars: list[str] = []
            for _ in range(width):
                if rng.random() < min(0.95, density * 0.45):
                    c = rng.choice(MATRIX_CHARS)
                    rv = rng.random()
                    if rv < 0.15:
                        chars.append(_tag(bright, c))
                    elif rv < 0.3:
                        chars.append(_tag(mid, c))
                    else:
                        chars.append(_tag(dim, c))
                else:
                    chars.append(" ")
            lines.append("".join(chars))
        return "\n".join(lines)

    def _render_music(self, cfg: dict) -> str:
        """Animated music visualizer bars with theme-aware colors."""
        bars = int(cfg.get("bars", 24))
        height = int(cfg.get("height", 6))
        sensitivity = float(cfg.get("sensitivity", 0.5))
        p = self.pal

        if self._theme_name == "light":
            low, mid, high = "#16a34a", "#ca8a04", "#dc2626"
        else:
            low, mid, high = "#00ff41", "#ffff00", "#ff4444"

        lines: list[str] = []
        for row in range(height):
            row_chars: list[str] = []
            threshold = (height - row) / height
            for i in range(bars):
                val = (__import__("math").sin((self._tick / 1.5) + i * 0.55) + 1) / 2
                amp = val * sensitivity * 1.8
                if amp >= threshold:
                    if row < height // 3:
                        row_chars.append(_tag(low, "█"))
                    elif row < height * 2 // 3:
                        row_chars.append(_tag(mid, "█"))
                    else:
                        row_chars.append(_tag(high, "█"))
                else:
                    row_chars.append(" ")
            lines.append("".join(row_chars))
        return "\n".join(lines)

    def apply_theme(self, theme: str) -> None:
        self._theme_name = theme
        is_light = theme == "light"
        if is_light:
            self.add_class("vanta-light")
        else:
            self.remove_class("vanta-light")
        # Force re-render immediately
        self._refresh()

    CSS = """
    #wg-body {
        padding: 0 1 1 1;
    }
    .wg-row {
        height: 1fr;
        margin-bottom: 1;
    }
    .wg-panel {
        width: 1fr;
        border: solid #1e1e3f;
        background: #0f0f1a;
        padding: 1;
        margin-right: 1;
        overflow: hidden;
    }
    .wg-panel:last-child {
        margin-right: 0;
    }
    .wg-info-panel {
        width: 1fr;
    }
    /* Light theme overrides */
    .vanta-light .wg-panel {
        border: solid #d1d5db;
        background: #ffffff;
    }
    """
