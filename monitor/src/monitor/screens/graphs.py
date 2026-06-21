from __future__ import annotations

from pathlib import Path

from textual.app import ComposeResult
from textual.binding import Binding
from textual.containers import Horizontal, Vertical
from textual.events import Resize
from textual.screen import Screen
from textual.widgets import Footer, Header, Static

from monitor.core.collectors import SystemCollector
from monitor.core.dashboard_config import load_dashboard_config
from monitor.core.graph_presenter import make_graph_label, time_range_label
from monitor.core.history import HistoryBuffer
from monitor.core.overview_presenter import format_rate_binary
from monitor.core.theme import get_palette, is_light_theme, theme_label

CONFIG_PATH = Path(__file__).resolve().parents[3] / "config.json"
STYLES_DIR = Path(__file__).resolve().parent.parent / "styles"
BARS = "▁▂▃▄▅▆▇█"


def _spark(values: list[float], width: int = 48) -> str:
    if not values:
        return "·" * width
    vals = values[-width:]
    lo = min(vals)
    hi = max(vals)
    rng = hi - lo or 1.0
    out = []
    for value in vals:
        idx = int((value - lo) / rng * (len(BARS) - 1))
        idx = max(0, min(len(BARS) - 1, idx))
        out.append(BARS[idx])
    if len(out) < width:
        out = ["·"] * (width - len(out)) + out
    return "".join(out)


class GraphsScreen(Screen):
    """Dedicated trends screen for CPU, memory, network throughput, and disk I/O."""

    BINDINGS = [
        Binding("escape", "noop", "Back", show=False),
    ]

    def __init__(self) -> None:
        super().__init__()
        self.collector = SystemCollector()
        self.dashboard_config = load_dashboard_config(CONFIG_PATH)
        self._theme_name = self.dashboard_config.ui.theme or "light"
        self._dom_ready = False
        self._refresh_timer = None
        self._last_error: str | None = None
        self._last_mode = "full"
        self._cpu_short = HistoryBuffer(size=60)
        self._cpu_long = HistoryBuffer(size=300)
        self._mem_short = HistoryBuffer(size=60)
        self._mem_long = HistoryBuffer(size=300)
        self._net_up_short = HistoryBuffer(size=60)
        self._net_up_long = HistoryBuffer(size=300)
        self._net_down_short = HistoryBuffer(size=60)
        self._net_down_long = HistoryBuffer(size=300)
        self._disk_read_short = HistoryBuffer(size=60)
        self._disk_read_long = HistoryBuffer(size=300)
        self._disk_write_short = HistoryBuffer(size=60)
        self._disk_write_long = HistoryBuffer(size=300)

    @property
    def pal(self) -> dict[str, str]:
        return get_palette(self._theme_name)

    def compose(self) -> ComposeResult:
        yield Header(show_clock=True)
        with Vertical(id="graphs-body"):
            yield Static(id="graphs-status", classes="graphs-status")
            with Horizontal(id="graphs-row-top"):
                yield Static(id="graph-cpu", classes="graph-panel")
                yield Static(id="graph-mem", classes="graph-panel")
            with Horizontal(id="graphs-row-bottom"):
                yield Static(id="graph-net", classes="graph-panel")
                yield Static(id="graph-disk", classes="graph-panel")
        yield Footer()

    def on_mount(self) -> None:
        self._dom_ready = True
        self._refresh_timer = self.set_interval(max(0.5, self.dashboard_config.ui.refresh_rate), self._refresh)
        self._reflow()
        self._refresh()

    def on_resize(self, _: Resize) -> None:
        self._reflow()
        self._refresh()

    def _reflow(self) -> None:
        height = self.size.height
        compact = self.dashboard_config.compact_mode_for_height(height)
        tiny = self.dashboard_config.ultra_compact_mode_for_height(height)
        self.set_class(compact and not tiny, "compact")
        self.set_class(tiny, "tiny")
        self._last_mode = "tiny" if tiny else "compact" if compact else "full"

    def _refresh(self) -> None:
        p = self.pal
        try:
            snap = self.collector.sample()
            self._last_error = None
        except Exception as exc:
            self._last_error = str(exc)
            if self._dom_ready:
                self.query_one("#graphs-status", Static).update(f"[{p['red']}]collector error: {exc}[/]")
            return

        if snap.disks:
            disk = snap.disks[0]
            disk_read = disk.io_read_bps
            disk_write = disk.io_write_bps
        else:
            disk_read = 0.0
            disk_write = 0.0

        self._cpu_short.push(snap.cpu.total_percent)
        self._cpu_long.push(snap.cpu.total_percent)
        self._mem_short.push(snap.memory.percent)
        self._mem_long.push(snap.memory.percent)
        self._net_up_short.push(snap.network.upload_bps)
        self._net_up_long.push(snap.network.upload_bps)
        self._net_down_short.push(snap.network.download_bps)
        self._net_down_long.push(snap.network.download_bps)
        self._disk_read_short.push(disk_read)
        self._disk_read_long.push(disk_read)
        self._disk_write_short.push(disk_write)
        self._disk_write_long.push(disk_write)

        self.query_one("#graph-cpu", Static).update(self._render_percent_panel(
            "CPU history",
            snap.cpu.total_percent,
            self._cpu_long.values(),
            self._cpu_short.values(),
            unit="%",
        ))
        self.query_one("#graph-mem", Static).update(self._render_percent_panel(
            "Memory history",
            snap.memory.percent,
            self._mem_long.values(),
            self._mem_short.values(),
            unit="%",
        ))
        self.query_one("#graph-net", Static).update(self._render_dual_rate_panel(
            up_now=snap.network.upload_bps,
            down_now=snap.network.download_bps,
            up_long=self._net_up_long.values(),
            down_long=self._net_down_long.values(),
        ))
        self.query_one("#graph-disk", Static).update(self._render_dual_rate_panel(
            title="Disk I/O",
            up_label="Read",
            down_label="Write",
            up_now=disk_read,
            down_now=disk_write,
            up_long=self._disk_read_long.values(),
            down_long=self._disk_write_long.values(),
        ))
        err = f"  [{p['red']}]err={self._last_error}[/]" if self._last_error else ""
        self.query_one("#graphs-status", Static).update(
            f"[{p['text_dim']}]mode={self._last_mode}  theme={theme_label(self._theme_name)}  "
            f"ranges={time_range_label(60)} / {time_range_label(300)}[/]{err}"
        )

    def _render_percent_panel(self, title: str, current: float, long_values: list[float], short_values: list[float], unit: str) -> str:
        p = self.pal
        dot_c = "green" if current < 50 else ("yellow" if current < 80 else "red")
        return "\n".join([
            f"[{p['accent']}]● [/]"
            f"[{p['text']}]{title:<16} "
            f"[{p[dot_c]}]{current:>5.1f}{unit}[/]",
            f"[{p['green']}]{_spark(long_values, 46)}[/]",
            f"[{p['text_dim']}]{make_graph_label(long_values)}  {time_range_label(300)}[/]",
            f"[{p['yellow']}]{_spark(short_values, 46)}[/]",
            f"[{p['text_dim']}]{make_graph_label(short_values)}  {time_range_label(60)}[/]",
        ])

    def _render_dual_rate_panel(
        self,
        *,
        up_now: float,
        down_now: float,
        up_long: list[float],
        down_long: list[float],
        title: str = "Network throughput",
        icon: str = "🌐",
        up_label: str = "Up",
        down_label: str = "Down",
    ) -> str:
        p = self.pal
        return "\n".join([
            f"[{p['accent']}]● [/]"
            f"[{p['text']}]{title}[/]",
            f"  [{p['green']}]{up_label:<6} {format_rate_binary(up_now)}[/]",
            f"  [{p['green']}]{_spark(up_long, 40)}[/]",
            f"  [{p['text_dim']}]{make_graph_label(up_long, bytestyle=True)}[/]",
            f"  [{p['yellow']}]{down_label:<6} {format_rate_binary(down_now)}[/]",
            f"  [{p['yellow']}]{_spark(down_long, 40)}[/]",
            f"  [{p['text_dim']}]{make_graph_label(down_long, bytestyle=True)}[/]",
        ])

    def action_toggle_app_theme(self) -> None:
        if hasattr(self, "app") and self.app:
            self.app.action_toggle_theme()

    def action_noop(self) -> None:
        return

    def apply_theme(self, theme: str) -> None:
        self._theme_name = theme
        if is_light_theme(theme):
            self.add_class("vanta-light")
        else:
            self.remove_class("vanta-light")
        if self._dom_ready:
            self._refresh()

    CSS_PATH = str(STYLES_DIR / "graphs.tcss")
