"""Larger detailed trend graphs screen — like btop's graph view."""

from textual.app import ComposeResult
from textual.containers import Horizontal, Vertical
from textual.screen import Screen
from textual.widgets import Footer, Header, Static, Sparkline

from monitor.core.collectors import SystemCollector
from monitor.core.graph_presenter import format_graph_header, make_graph_label, time_range_label
from monitor.core.history import HistoryBuffer


class GraphScreen(Screen):
    """Dedicated screen with larger, detailed trend sparklines."""

    def __init__(self):
        super().__init__()
        self.collector = SystemCollector()

        # Short history (60s)
        self._cpu_60 = HistoryBuffer(size=60)
        self._mem_60 = HistoryBuffer(size=60)
        self._net_up_60 = HistoryBuffer(size=60)
        self._net_down_60 = HistoryBuffer(size=60)
        self._disk_60 = HistoryBuffer(size=60)

        # Long history (600s)
        self._cpu_600 = HistoryBuffer(size=600)
        self._mem_600 = HistoryBuffer(size=600)
        self._net_up_600 = HistoryBuffer(size=600)
        self._net_down_600 = HistoryBuffer(size=600)
        self._disk_600 = HistoryBuffer(size=600)

        self._refresh_timer = None
        self._current_cpu = 0.0
        self._current_mem = 0.0
        self._current_net_up = 0.0
        self._current_net_down = 0.0
        self._current_disk = 0.0

    def compose(self) -> ComposeResult:
        yield Header(show_clock=True)

        with Vertical(id="graphs-body"):
            # Row 1
            with Horizontal(classes="gr-row"):
                # CPU
                with Vertical(classes="gr-panel cpu-panel"):
                    yield Static(id="gr-cpu-title", classes="gr-title")
                    yield Sparkline([], id="gr-cpu-long", classes="gr-spark-long")
                    yield Sparkline([], id="gr-cpu-short", classes="gr-spark-short")
                    yield Static(id="gr-cpu-meta", classes="gr-meta")
                # Memory
                with Vertical(classes="gr-panel mem-panel"):
                    yield Static(id="gr-mem-title", classes="gr-title")
                    yield Sparkline([], id="gr-mem-long", classes="gr-spark-long")
                    yield Sparkline([], id="gr-mem-short", classes="gr-spark-short")
                    yield Static(id="gr-mem-meta", classes="gr-meta")
                # Network
                with Vertical(classes="gr-panel net-panel"):
                    yield Static(id="gr-net-title", classes="gr-title")
                    yield Sparkline([], id="gr-net-up", classes="gr-spark-long")
                    yield Sparkline([], id="gr-net-down", classes="gr-spark-short")
                    yield Static(id="gr-net-meta", classes="gr-meta")
            # Row 2
            with Horizontal(classes="gr-row"):
                # Disk
                with Vertical(classes="gr-panel disk-panel"):
                    yield Static(id="gr-disk-title", classes="gr-title")
                    yield Sparkline([], id="gr-disk-long", classes="gr-spark-long")
                    yield Sparkline([], id="gr-disk-short", classes="gr-spark-short")
                    yield Static(id="gr-disk-meta", classes="gr-meta")
                # Per-core CPU summary placeholder
                with Vertical(classes="gr-panel system-panel"):
                    yield Static(id="gr-system-title", classes="gr-title")
                    yield Sparkline([], id="gr-system-spark", classes="gr-spark-long")
                    yield Static(id="gr-system-info", classes="gr-meta")

        yield Footer()

    def on_mount(self) -> None:
        self._refresh_timer = self.set_interval(1.0, self._refresh)
        self._refresh()

    def _refresh(self) -> None:
        try:
            snap = self.collector.sample()
        except Exception:
            return

        cpu = snap.cpu.total_percent
        mem = snap.memory.percent
        net_up = snap.network.upload_bps / (1024**2)
        net_down = snap.network.download_bps / (1024**2)
        disk = snap.disks[0].percent if snap.disks else 0.0

        self._current_cpu = cpu
        self._current_mem = mem
        self._current_net_up = net_up
        self._current_net_down = net_down
        self._current_disk = disk

        # Push short
        self._cpu_60.push(cpu)
        self._mem_60.push(mem)
        self._net_up_60.push(net_up)
        self._net_down_60.push(net_down)
        self._disk_60.push(disk)

        # Push long
        self._cpu_600.push(cpu)
        self._mem_600.push(mem)
        self._net_up_600.push(net_up)
        self._net_down_600.push(net_down)
        self._disk_600.push(disk)

        # Update sparklines
        self.query_one("#gr-cpu-long", Sparkline).data = self._cpu_600.values()
        self.query_one("#gr-cpu-short", Sparkline).data = self._cpu_60.values()
        self.query_one("#gr-mem-long", Sparkline).data = self._mem_600.values()
        self.query_one("#gr-mem-short", Sparkline).data = self._mem_60.values()
        self.query_one("#gr-net-up", Sparkline).data = self._net_up_600.values()
        self.query_one("#gr-net-down", Sparkline).data = self._net_down_60.values()
        self.query_one("#gr-disk-long", Sparkline).data = self._disk_600.values()
        self.query_one("#gr-disk-short", Sparkline).data = self._disk_60.values()
        self.query_one("#gr-system-spark", Sparkline).data = self._cpu_600.values()

        # Update titles
        self.query_one("#gr-cpu-title", Static).update(format_graph_header("CPU", cpu, unit="%"))
        self.query_one("#gr-mem-title", Static).update(format_graph_header("Memory", mem, unit="%"))
        self.query_one(
            "#gr-net-title", Static
        ).update(
            f"{'Net up':<12} {net_up:>6.1f} MiB/s\n{'Net down':<12} {net_down:>6.1f} MiB/s"
        )
        self.query_one("#gr-disk-title", Static).update(format_graph_header("Disk", disk, unit="%"))

        # Update meta labels
        lo_cpu, hi_cpu = (min(self._cpu_600.values()), max(self._cpu_600.values())) if self._cpu_600.values() else (0, 100)
        self.query_one("#gr-cpu-meta", Static).update(
            f"10m range: {lo_cpu:.1f}–{hi_cpu:.1f}%"
        )
        self.query_one("#gr-mem-meta", Static).update(
            f"10m range: {min(self._mem_600.values()):.1f}–{max(self._mem_600.values()):.1f}%"
            if self._mem_600.values() else ""
        )
        self.query_one("#gr-net-meta", Static).update(
            f"↑ max {max(self._net_up_600.values()):.1f} MiB/s  ↓ max {max(self._net_down_600.values()):.1f} MiB/s"
            if self._net_up_600.values() else ""
        )
        self.query_one("#gr-disk-meta", Static).update(
            f"10m range: {min(self._disk_600.values()):.1f}–{max(self._disk_600.values()):.1f}%"
            if self._disk_600.values() else ""
        )

        # System info
        self.query_one("#gr-system-title", Static).update(
            f"System — {snap.cpu.core_count} cores"
        )
        self.query_one("#gr-system-info", Static).update(
            f"Load: {snap.cpu.load_avg_1m:.2f}  "
            f"Procs: {snap.process_count}" + (
                f"  Temp: {snap.temperature_c:.1f}°C" if snap.temperature_c else ""
            )
        )

    def apply_theme(self, theme: str) -> None:
        is_light = theme == "light"
        if is_light:
            self.add_class("vanta-light")
        else:
            self.remove_class("vanta-light")

    CSS = """
    #graphs-body {
        padding: 0 1 1 1;
    }
    .gr-row {
        height: 1fr;
        margin-bottom: 1;
    }
    .gr-panel {
        width: 1fr;
        border: solid #1e1e3f;
        background: #0f0f1a;
        padding: 1;
        margin-right: 1;
    }
    .gr-panel:last-child {
        margin-right: 0;
    }
    .gr-title {
        height: 2;
        color: #cbd5e1;
        text-style: bold;
        margin-bottom: 1;
    }
    .gr-spark-long {
        height: 7;
    }
    .gr-spark-short {
        height: 4;
        margin-bottom: 1;
    }
    .gr-meta {
        height: 1;
        color: #64748b;
    }
    
    .vanta-light .gr-panel {
        border: solid #d1d5db;
        background: #ffffff;
    }
    .vanta-light .gr-title {
        color: #1a1a1a;
    }
    .vanta-light .gr-meta {
        color: #6b7280;
    }
    """
