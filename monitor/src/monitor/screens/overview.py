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
from monitor.core.dashboard_widgets import WidgetRenderCache, paginate_widgets
from monitor.core.history import HistoryBuffer
from monitor.core.overview_presenter import make_overview_panels, make_process_preview
from monitor.core.process_service import ProcessService

CONFIG_PATH = Path(__file__).resolve().parents[3] / "config.json"


class OverviewScreen(Screen):
    """Config-driven modular dashboard with dense system monitor core."""

    BINDINGS = [
        Binding("[", "prev_widget_page", "Prev widgets", show=False),
        Binding("]", "next_widget_page", "Next widgets", show=False),
    ]

    def __init__(self):
        super().__init__()
        self.collector = SystemCollector()
        self.process_service = ProcessService()
        self.dashboard_config = load_dashboard_config(CONFIG_PATH)
        self._widget_pages = [[]]
        self._widget_page = 0
        self._widget_slots_visible = 3
        self._widget_cache = WidgetRenderCache()
        self._cpu_hist = HistoryBuffer(size=120)
        self._mem_hist = HistoryBuffer(size=120)
        self._net_up_hist = HistoryBuffer(size=120)
        self._net_down_hist = HistoryBuffer(size=120)
        self._disk_hist = HistoryBuffer(size=120)
        self._refresh_timer = None
        self._process_count = 0
        self._active_widget_names: list[str] = []
        self._last_error: str | None = None
        self._theme_name = "light"

    @property
    def pal(self) -> dict[str, str]:
        return LIGHT if self._theme_name == "light" else DARK

    def compose(self) -> ComposeResult:
        yield Header(show_clock=True)
        with Vertical(id="overview-body"):
            yield Static(id="dashboard-status", classes="status-strip")
            with Horizontal(id="overview-top"):
                yield Static(id="cpu-panel", classes="ov-panel left-panel")
                with Vertical(id="overview-right"):
                    yield Static(id="memory-panel", classes="ov-panel right-panel")
                    yield Static(id="network-panel", classes="ov-panel right-panel")
                    yield Static(id="system-panel", classes="ov-panel right-panel")
            with Horizontal(id="overview-bottom"):
                with Vertical(id="overview-bottom-left"):
                    yield Static("Disks", classes="ov-title")
                    yield Static(id="disk-panel", classes="ov-panel disk-panel")
                with Vertical(id="overview-bottom-right"):
                    yield Static("Top processes", classes="ov-title")
                    yield Static(id="proc-panel", classes="ov-panel proc-panel")
            yield Static(id="widget-tray-title", classes="ov-title")
            with Horizontal(id="widget-tray"):
                yield Static(id="widget-slot-0", classes="ov-panel widget-panel")
                yield Static(id="widget-slot-1", classes="ov-panel widget-panel")
                yield Static(id="widget-slot-2", classes="ov-panel widget-panel")
                yield Static(id="widget-slot-3", classes="ov-panel widget-panel")
            with Horizontal(id="history-row"):
                yield Vertical(
                    Static("CPU history", classes="hist-title"),
                    Sparkline([], id="cpu-spark", classes="ov-spark"),
                    id="hist-cpu-col",
                    classes="hist-col",
                )
                yield Vertical(
                    Static("Mem history", classes="hist-title"),
                    Sparkline([], id="mem-spark", classes="ov-spark"),
                    id="hist-mem-col",
                    classes="hist-col",
                )
                with Vertical(classes="hist-col", id="hist-net-col"):
                    yield Static("Net up", classes="hist-title")
                    yield Sparkline([], id="net-up-spark", classes="ov-spark-short")
                    yield Static("Net down", classes="hist-title")
                    yield Sparkline([], id="net-down-spark", classes="ov-spark-short")
                yield Vertical(
                    Static("Disk use", classes="hist-title"),
                    Sparkline([], id="disk-spark", classes="ov-spark"),
                    id="hist-disk-col",
                    classes="hist-col",
                )
        yield Footer()

    def on_mount(self):
        self._reload_dashboard_config()
        interval = max(0.25, self.dashboard_config.ui.refresh_rate)
        self._refresh_timer = self.set_interval(interval, self._refresh)
        self._refresh()

    def on_resize(self, _: Resize) -> None:
        self._reflow_dashboard()
        self._refresh_widget_slots()
        self._update_status_strip()

    def _reload_dashboard_config(self) -> None:
        self.dashboard_config = load_dashboard_config(CONFIG_PATH)
        self._reflow_dashboard()
        self._widget_pages = paginate_widgets(
            self.dashboard_config.enabled_extra_widget_names(),
            page_size=self._widget_slots_visible,
        )
        if self._widget_page >= len(self._widget_pages):
            self._widget_page = max(0, len(self._widget_pages) - 1)

    def _reflow_dashboard(self) -> None:
        width = max(1, self.size.width or 120)
        height = max(1, self.size.height or 40)
        self._widget_slots_visible = self.dashboard_config.page_size_for_width(width)
        history_visible = self.dashboard_config.show_history_for_height(height)
        compact_mode = self.dashboard_config.compact_mode_for_height(height)
        ultra_compact_mode = self.dashboard_config.ultra_compact_mode_for_height(height)

        history_row = self.query("#history-row")
        if history_row:
            history_row.first().display = history_visible and not ultra_compact_mode
        widget_title = self.query("#widget-tray-title")
        widget_tray = self.query("#widget-tray")
        if widget_title and widget_tray:
            widget_title.first().display = not ultra_compact_mode
            widget_tray.first().display = not ultra_compact_mode
        if ultra_compact_mode:
            self.query_one("#overview-top", Horizontal).styles.height = 10
            self.query_one("#overview-bottom", Horizontal).styles.height = 6
            self.query_one("#widget-tray", Horizontal).styles.height = 0
        elif compact_mode:
            self.query_one("#overview-top", Horizontal).styles.height = 12
            self.query_one("#overview-bottom", Horizontal).styles.height = 8
            self.query_one("#widget-tray", Horizontal).styles.height = 7
        else:
            self.query_one("#overview-top", Horizontal).styles.height = 14
            self.query_one("#overview-bottom", Horizontal).styles.height = 10
            self.query_one("#widget-tray", Horizontal).styles.height = 9
        for index in range(4):
            slot = self.query_one(f"#widget-slot-{index}", Static)
            slot.display = (index < self._widget_slots_visible) and not ultra_compact_mode

    def _refresh(self):
        self._reload_dashboard_config()
        dashboard_cfg = self.dashboard_config.widget("dashboard")
        process_cfg = self.dashboard_config.widget("process_manager")
        dashboard_enabled = dashboard_cfg.enabled
        process_enabled = process_cfg.enabled

        try:
            snap = self.collector.sample()
            processes = (
                self.process_service.list_processes(
                    include_kernel=bool(process_cfg.get("show_kernel", self.dashboard_config.process.show_kernel)),
                    sort_by="cpu",
                    descending=True,
                    limit=int(process_cfg.get("max_display", self.dashboard_config.process.max_display)),
                )
                if process_enabled
                else []
            )
            self._last_error = None
        except Exception as exc:
            self._last_error = str(exc)
            self._update_status_strip()
            return

        self._process_count = len(processes)
        self._active_widget_names = self.dashboard_config.enabled_extra_widget_names()

        self._cpu_hist.push(snap.cpu.total_percent)
        self._mem_hist.push(snap.memory.percent)
        self._net_up_hist.push(snap.network.upload_bps / (1024**2))
        self._net_down_hist.push(snap.network.download_bps / (1024**2))
        if snap.disks:
            self._disk_hist.push(snap.disks[0].percent)

        if dashboard_enabled:
            panels = make_overview_panels(snap)
            self.query_one("#cpu-panel", Static).update(panels["cpu"])
            self.query_one("#memory-panel", Static).update(panels["memory"])
            self.query_one("#network-panel", Static).update(panels["network"])
            self.query_one("#system-panel", Static).update(panels["system"])
            self.query_one("#disk-panel", Static).update(panels["disks"])
        else:
            disabled = "[disabled in config]"
            self.query_one("#cpu-panel", Static).update(f"CPU\n{disabled}")
            self.query_one("#memory-panel", Static).update(f"Memory\n{disabled}")
            self.query_one("#network-panel", Static).update(f"Network\n{disabled}")
            self.query_one("#system-panel", Static).update(f"System\n{disabled}")
            self.query_one("#disk-panel", Static).update(f"Disks\n{disabled}")

        proc_limit = int(process_cfg.get("max_display", self.dashboard_config.process.max_display))
        proc_text = make_process_preview(processes, limit=proc_limit) if process_enabled else "Process manager disabled in config"
        self.query_one("#proc-panel", Static).update(proc_text)

        self.query_one("#cpu-spark", Sparkline).data = self._cpu_hist.values()
        self.query_one("#mem-spark", Sparkline).data = self._mem_hist.values()
        self.query_one("#net-up-spark", Sparkline).data = self._net_up_hist.values()
        self.query_one("#net-down-spark", Sparkline).data = self._net_down_hist.values()
        self.query_one("#disk-spark", Sparkline).data = self._disk_hist.values()

        self._refresh_widget_slots()
        self._update_status_strip()

    def _update_status_strip(self) -> None:
        width = max(1, self.size.width or 120)
        height = max(1, self.size.height or 40)
        if self.dashboard_config.ultra_compact_mode_for_height(height):
            mode = "tiny"
        elif self.dashboard_config.compact_mode_for_height(height):
            mode = "compact"
        else:
            mode = "full"
        page_count = max(1, len(self._widget_pages))
        widget_count = len(self._active_widget_names or self.dashboard_config.enabled_extra_widget_names())
        error = f"  err={self._last_error}" if self._last_error else ""
        text = (
            f"mode={mode}  term={width}x{height}  refresh={self.dashboard_config.ui.refresh_rate:.2f}s  "
            f"widgets={widget_count}  page={self._widget_page + 1}/{page_count}  slots={self._widget_slots_visible}  "
            f"procs={self._process_count}{error}"
        )
        self.query_one("#dashboard-status", Static).update(text)

    def _refresh_widget_slots(self) -> None:
        pages = self._widget_pages or [[]]
        page = pages[self._widget_page] if pages else []
        total_pages = len(pages)
        all_enabled = self.dashboard_config.enabled_extra_widget_names()
        title = (
            f"Widget dock  {self._widget_page + 1}/{total_pages}  [[]/[]] cycle  enabled: {', '.join(all_enabled)}"
            if all_enabled
            else "Widget dock — no extra widgets enabled in config"
        )
        self.query_one("#widget-tray-title", Static).update(title)
        for index in range(4):
            slot = self.query_one(f"#widget-slot-{index}", Static)
            if index < len(page):
                name = page[index]
                cfg = self.dashboard_config.widget(name).settings
                title_name = name.replace("_", " ").title()
                content = self._widget_cache.render(name, cfg)
                slot.update(f"[{self.pal['text_muted']}]{title_name}[/]\n{content}")
            else:
                slot.update(f"[{self.pal['text_dim']}]empty slot[/]")

    def action_prev_widget_page(self) -> None:
        if self._widget_pages:
            self._widget_page = (self._widget_page - 1) % len(self._widget_pages)
            self._refresh_widget_slots()
            self._update_status_strip()

    def action_next_widget_page(self) -> None:
        if self._widget_pages:
            self._widget_page = (self._widget_page + 1) % len(self._widget_pages)
            self._refresh_widget_slots()
            self._update_status_strip()

    def apply_theme(self, theme: str) -> None:
        self._theme_name = theme
        is_light = theme == "light"
        if is_light:
            self.add_class("vanta-light")
        else:
            self.remove_class("vanta-light")
        # Force widget slot re-render with correct colors
        self._refresh_widget_slots()

    CSS = """ 
    #overview-body {
        padding: 0 1 1 1;
    }
    .status-strip {
        height: 1;
        color: #94a3b8;
        background: #0d1320;
        border: tall #1e1e3f;
        padding: 0 1;
        margin-bottom: 1;
    }
    #overview-top {
        height: 14;
        margin-bottom: 1;
    }
    #overview-bottom {
        height: 10;
        margin-bottom: 1;
    }
    #overview-right {
        width: 38;
    }
    #overview-bottom-left, #overview-bottom-right {
        width: 1fr;
    }
    .ov-title {
        color: #64748b;
        text-style: bold;
        margin-left: 1;
    }
    .ov-panel {
        border: solid #1e1e3f;
        background: #0f0f1a;
        padding: 1;
        color: #cbd5e1;
    }
    .left-panel {
        width: 1fr;
        margin-right: 1;
    }
    .right-panel {
        height: 1fr;
        margin-bottom: 1;
    }
    .disk-panel {
        height: 1fr;
        margin-right: 1;
    }
    .proc-panel {
        height: 1fr;
    }
    #widget-tray {
        height: 9;
        margin-bottom: 1;
    }
    .widget-panel {
        width: 1fr;
        margin-right: 1;
        overflow: hidden;
    }
    .widget-panel:last-child {
        margin-right: 0;
    }
    #history-row {
        height: 8;
    }
    .hist-col {
        width: 1fr;
        border: solid #1e1e3f;
        background: #0f0f1a;
        padding: 0 1;
        margin-right: 1;
    }
    .hist-col:last-child {
        margin-right: 0;
    }
    .hist-title {
        text-align: center;
        color: #64748b;
        text-style: bold;
        height: 1;
    }
    .ov-spark {
        height: 5;
    }
    .ov-spark-short {
        height: 3;
    }

    /* Light theme overrides */
    .vanta-light .status-strip {
        color: #6b7280;
        background: #ffffff;
        border: tall #d1d5db;
    }
    .vanta-light .ov-title, .vanta-light .hist-title {
        color: #6b7280;
    }
    .vanta-light .ov-panel, .vanta-light .hist-col {
        border: solid #d1d5db;
        background: #ffffff;
        color: #1a1a1a;
    }
    """
