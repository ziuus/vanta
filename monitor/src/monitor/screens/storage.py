from textual.app import ComposeResult
from textual.screen import Screen
from textual.widgets import Header, Footer, Static, DataTable
from textual.containers import Vertical
from textual.binding import Binding

from monitor.core.theme import DARK, LIGHT
from monitor.core.collectors import SystemCollector


class StorageScreen(Screen):
    """Storage mount overview."""

    BINDINGS = [
        Binding("r", "refresh", "Refresh"),
    ]

    def __init__(self):
        super().__init__()
        self.collector = SystemCollector()
        self._refresh_timer = None

    def compose(self) -> ComposeResult:
        yield Header(show_clock=True)
        with Vertical(id="storage-body"):
            yield DataTable(id="storage-table")
        yield Footer()

    def on_mount(self):
        table = self.query_one("#storage-table", DataTable)
        table.add_columns("Mount", "Total", "Used", "Free", "Use%")
        table.cursor_type = "row"
        self._refresh_timer = self.set_interval(5.0, self._refresh)
        self._refresh()

    def _refresh(self):
        try:
            snap = self.collector.sample()
        except Exception:
            return
        table = self.query_one("#storage-table", DataTable)
        table.clear()
        for d in snap.disks:
            total_gb = d.total_bytes / 1e9
            used_gb = d.used_bytes / 1e9
            free_gb = d.free_bytes / 1e9
            table.add_row(
                d.mountpoint,
                f"{total_gb:.1f} GB",
                f"{used_gb:.1f} GB",
                f"{free_gb:.1f} GB",
                f"{d.percent:.1f}%",
            )

    def action_refresh(self):
        self._refresh()

    def apply_theme(self, theme: str) -> None:
        is_light = theme == "light"
        if is_light:
            self.add_class("vanta-light")
        else:
            self.remove_class("vanta-light")

    CSS = """
    #storage-body {
        padding: 1 2;
    }
    #storage-table {
        height: 1fr;
    }
    DataTable {
        border: solid #1e1e3f;
        background: #0f0f1a;
    }
    DataTable > .datatable--header {
        background: #1a1a2e;
        color: #64748b;
    }
    DataTable > .datatable--cursor {
        background: #1e1e3f;
    }
    
    .vanta-light DataTable {
        border: solid #d1d5db;
        background: #ffffff;
    }
    .vanta-light DataTable > .datatable--header {
        background: #e8ecf0;
        color: #6b7280;
    }
    .vanta-light DataTable > .datatable--cursor {
        background: #d1d5db;
    }
    """
