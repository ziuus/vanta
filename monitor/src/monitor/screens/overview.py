from __future__ import annotations

from pathlib import Path

from textual.app import ComposeResult
from textual.containers import Horizontal, Vertical
from textual.screen import Screen
from textual.widgets import Footer, Header, Static, Sparkline
from textual.binding import Binding
from textual.events import Resize

from monitor.core.theme import DARK, LIGHT
from monitor.core.collectors import SystemCollector
from monitor.core.dashboard_config import DashboardConfig, load_dashboard_config
from monitor.core.dashboard_widgets import WidgetRenderCache
from monitor.core.history import HistoryBuffer
from monitor.core.media import MediaDetector
from monitor.core.overview_presenter import (
    make_overview_panels,
    make_process_preview,
    format_rate_binary,
)
from monitor.core.process_service import ProcessService

CONFIG_PATH = Path(__file__).resolve().parents[3] / "config.json"
BARS = "▁▂▃▄▅▆▇█"


class OverviewScreen(Screen):
    """Unified system dashboard — stats, media, widgets, everything on one screen."""

    BINDINGS = [
        Binding("w", "cycle_widget", "Widgets", show=False),
    ]

    def __init__(self):
        super().__init__()
        self.collector = SystemCollector()
        self.process_service = ProcessService()
        self.dashboard_config = load_dashboard_config(CONFIG_PATH)
        self.media = MediaDetector()
        self._widget_cache = WidgetRenderCache()
        self._cpu_hist = HistoryBuffer(size=80)
        self._mem_hist = HistoryBuffer(size=80)
        self._net_up_hist = HistoryBuffer(size=80)
        self._net_down_hist = HistoryBuffer(size=80)
        self._disk_hist = HistoryBuffer(size=80)
        self._last_snap = None
        self._refresh_timer = None
        self._process_count = 0
        self._last_error: str | None = None
        self._theme_name = "light"
        self._widget_index = 0
        self._widget_names = [
            n for n in ["clock", "calendar", "matrix", "music_viz"]
            if self.dashboard_config.widget(n).enabled
        ]
        if not self._widget_names:
            self._widget_names = ["clock"]

    @property
    def pal(self) -> dict[str, str]:
        return LIGHT if self._theme_name == "light" else DARK

    def compose(self) -> ComposeResult:
        yield Header(show_clock=True)
        with Vertical(id="dash-body"):
            # Compact status strip — one line of key metrics
            yield Static(id="dash-status", classes="status-strip")
            # Five-panel stats row
            with Horizontal(id="dash-panels"):
                yield Static(id="cpu-panel", classes="panel")
                yield Static(id="memory-panel", classes="panel")
                yield Static(id="network-panel", classes="panel")
                yield Static(id="disk-panel", classes="panel")
                yield Static(id="nowplaying-panel", classes="panel")
            # Bottom: processes (left) | widget (right)
            with Horizontal(id="dash-bottom"):
                yield Static(id="proc-panel", classes="proc-box")
                with Vertical(id="dash-widget-col"):
                    yield Static(id="dash-widget-title", classes="widget-ttl")
                    yield Static(id="dash-widget", classes="widget-box")
        yield Footer()

    def on_mount(self):
        self._dom_ready = True
        interval = max(0.5, self.dashboard_config.ui.refresh_rate)
        self._refresh_timer = self.set_interval(interval, self._refresh)
        self._refresh()

    def on_resize(self, _: Resize) -> None:
        self._refresh()

    def _refresh(self):
        p = self.pal
        try:
            snap = self.collector.sample()
            processes = self.process_service.list_processes(
                include_kernel=bool(
                    self.dashboard_config.widget("process_manager").get(
                        "show_kernel", self.dashboard_config.process.show_kernel
                    )
                ),
                sort_by="cpu", descending=True,
                limit=int(
                    self.dashboard_config.widget("process_manager").get(
                        "max_display", self.dashboard_config.process.max_display
                    )
                ),
            )
            self._last_snap = snap
            self._last_error = None
        except Exception as exc:
            self._last_error = str(exc)
            self._update_status()
            return

        self._process_count = len(processes)

        # Push histories
        self._cpu_hist.push(snap.cpu.total_percent)
        self._mem_hist.push(snap.memory.percent)
        self._net_up_hist.push(snap.network.upload_bps / (1024**2))
        self._net_down_hist.push(snap.network.download_bps / (1024**2))
        if snap.disks:
            self._disk_hist.push(snap.disks[0].percent)

        # --- STAT PANELS (left 4/5 of top) ---
        panels = make_overview_panels(snap)
        self.query_one("#cpu-panel", Static).update(panels["cpu"])
        self.query_one("#memory-panel", Static).update(panels["memory"])
        self.query_one("#network-panel", Static).update(panels["network"])
        self.query_one("#disk-panel", Static).update(panels["disks"])

        # --- NOW PLAYING (right 1/5 of top) ---
        np_info = self.media.detect()
        if np_info and np_info["status"] != "Stopped":
            status_icon = "▶" if np_info["status"] == "Playing" else "⏸"
            title = np_info["title"][:24]
            artist = np_info["artist"][:24]
            album_line = ""
            if np_info.get("album"):
                album_line = f"\n[{p['text_dim']}]{np_info['album'][:24]}[/]"
            # Animated bars
            from datetime import datetime
            import math
            t = datetime.now().timestamp()
            bar_count = 12
            bars_chars = []
            for i in range(bar_count):
                val = (math.sin(t + i * 0.55) + 1) / 2
                idx = min(len(BARS) - 1, int(val * (len(BARS) - 1)))
                bars_chars.append(BARS[idx])
            bars_str = "".join(bars_chars)
            np_text = (
                f"[{p['accent']}]{status_icon} Now Playing[/]\n"
                f"[{p['text']}]{title}[/]\n"
                f"[{p['text_muted']}]{artist}[/]{album_line}\n"
                f"[{p['green']}]{bars_str}[/]"
            )
        else:
            np_text = (
                f"[{p['text_muted']}]▶ No media[/]\n"
                f"[{p['text_dim']}]Launch spotify, mpv, or[/]\n"
                f"[{p['text_dim']}]vlc to see playback[/]"
            )
        self.query_one("#nowplaying-panel", Static).update(np_text)

        # --- TOP PROCESSES (left column, bottom) ---
        proc_limit = int(
            self.dashboard_config.widget("process_manager").get(
                "max_display", self.dashboard_config.process.max_display
            )
        )
        self.query_one("#proc-panel", Static).update(
            make_process_preview(processes, limit=min(proc_limit, 12))
        )

        # --- WIDGET (right column, bottom) ---
        self._render_dash_widget()

        # --- STATUS STRIP ---
        self._update_status()

    def _render_dash_widget(self):
        p = self.pal
        if not self._widget_names:
            self.query_one("#dash-widget", Static).update("")
            self.query_one("#dash-widget-title", Static).update("")
            return
        name = self._widget_names[self._widget_index % len(self._widget_names)]
        cfg = self.dashboard_config.widget(name).settings
        content = self._widget_cache.render(name, cfg)
        title = name.replace("_", " ").title()
        total = len(self._widget_names)
        idx = self._widget_index + 1
        self.query_one("#dash-widget-title", Static).update(
            f"[{p['text_muted']}]● {title}  [{p['text_dim']}]{idx}/{total}[/]  "
            f"[{p['text_dim']}]press W to cycle[/][/]"
        )
        self.query_one("#dash-widget", Static).update(content)

    def _update_status(self):
        p = self.pal
        snap = getattr(self, "_last_snap", None)
        error = f"  [{p['red']}]err={self._last_error}[/]" if self._last_error else ""

        cpu_s = ""
        mem_s = ""
        net_s = ""
        disk_s = ""
        if snap:
            cpu_v = self._cpu_hist.values()
            mem_v = self._mem_hist.values()
            net_v = self._net_up_hist.values()
            disk_v = self._disk_hist.values()

            def spark(vals: list[float], w: int = 6) -> str:
                if len(vals) < 2:
                    return "─" * w
                mn, mx = min(vals), max(vals)
                rng = mx - mn if mx > mn else 1
                return "".join(BARS[min(len(BARS) - 1, int((v - mn) / rng * (len(BARS) - 1)))] for v in vals[-w:])

            cpu_s = f"CPU {snap.cpu.total_percent:.0f}% {spark(cpu_v)}"
            mem_s = f"MEM {snap.memory.percent:.0f}% {spark(mem_v)}"
            net_s = f"NET ↑{format_rate_binary(snap.network.upload_bps).replace(' ', '')} ↓{format_rate_binary(snap.network.download_bps).replace(' ', '')}"
            disk_s = f"DISK {snap.disks[0].percent:.0f}% {spark(disk_v)}" if snap.disks else ""

        uptime_h = snap.uptime_seconds / 3600.0 if snap else 0
        uptime_s = f"Up {uptime_h:.1f}h" if uptime_h else ""

        text = (
            f"[{p['text']}]{cpu_s}  {mem_s}  {net_s}  {disk_s}"
            f"  [{p['text_dim']}]{uptime_s}  procs={self._process_count}{error}[/]"
        ) if snap else f"[{p['text_dim']}]Waiting for data...[/]"
        self.query_one("#dash-status", Static).update(text)

    def action_cycle_widget(self):
        if self._widget_names:
            self._widget_index = (self._widget_index + 1) % len(self._widget_names)
        self._render_dash_widget()

    def apply_theme(self, theme: str) -> None:
        self._theme_name = theme
        is_light = theme == "light"
        if is_light:
            self.add_class("vanta-light")
        else:
            self.remove_class("vanta-light")
        # Refresh only if the screen has already been composed (widgets exist)
        if hasattr(self, "_dom_ready") and self._dom_ready:
            self._refresh()

    CSS = """
    #dash-body {
        padding: 0 1 1 1;
    }
    /* --- Status strip --- */
    .status-strip {
        height: 1;
        color: #94a3b8;
        background: #0d1320;
        border: tall #1e1e3f;
        padding: 0 1;
        margin-bottom: 1;
    }
    /* --- Five-panel stats row --- */
    #dash-panels {
        height: 11;
        margin-bottom: 1;
    }
    .panel {
        width: 1fr;
        margin-right: 1;
        border: solid #1e1e3f;
        background: #0f0f1a;
        padding: 0 1;
        color: #cbd5e1;
    }
    .panel:last-child {
        margin-right: 0;
    }
    /* --- Bottom row: processes + widget --- */
    #dash-bottom {
        height: 12;
        margin-bottom: 0;
    }
    .proc-box {
        width: 2fr;
        margin-right: 1;
        border: solid #1e1e3f;
        background: #0f0f1a;
        padding: 0 1;
        color: #cbd5e1;
    }
    #dash-widget-col {
        width: 1fr;
    }
    .widget-ttl {
        height: 1;
        color: #64748b;
        text-style: bold;
    }
    .widget-box {
        height: 1fr;
        border: solid #1e1e3f;
        background: #0f0f1a;
        padding: 0 1;
        color: #cbd5e1;
    }

    /* === LIGHT THEME === */
    .vanta-light .status-strip {
        color: #6b7280;
        background: #ffffff;
        border: tall #d1d5db;
    }
    .vanta-light .panel,
    .vanta-light .proc-box,
    .vanta-light .widget-box {
        border: solid #d1d5db;
        background: #ffffff;
        color: #1a1a1a;
    }
    .vanta-light .widget-ttl {
        color: #6b7280;
    }
    """
