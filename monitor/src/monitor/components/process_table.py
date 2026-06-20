"""Reusable DataTable-based process list with keyboard actions."""
from rich.text import Text
from textual.app import ComposeResult
from textual.containers import Horizontal, Vertical
from textual.widgets import DataTable, Static, Input
from textual.widget import Widget
from textual.message import Message

from monitor.core.process_presenter import format_process_detail, format_process_status
from monitor.core.process_service import ProcessService, next_sort_column


def _pct_color(value: float) -> str:
    """Return Rich color name for a utilization percentage."""
    if value < 50:
        return "green"
    if value < 80:
        return "yellow"
    return "red"


_SORT_MARKER = " ▲"  # asc or desc applied dynamically


class ProcessTable(Widget):
    """An auto-refreshing process DataTable with kill/suspend/resume."""

    class ProcessSelected(Message):
        """Emitted when a process action is triggered."""

        def __init__(self, pid: int, name: str, action: str) -> None:
            super().__init__()
            self.pid = pid
            self.name = name
            self.action = action

    def __init__(self, svc: ProcessService | None = None, **kwargs):
        super().__init__(**kwargs)
        self.svc = svc or ProcessService()
        self._timer = None
        self._sort_col = "cpu"
        self._descending = True
        self._rows = []

    def compose(self) -> ComposeResult:
        with Vertical():
            with Horizontal(classes="pt-toolbar"):
                yield Input(placeholder="Filter...", id="pt-filter", classes="pt-input")
                yield Static("k=kill  s=stop  r=resume  t=next sort  Ctrl+T=toggle dir", classes="pt-hint")
            yield Static("No process selected", id="pt-detail", classes="pt-detail")
            yield Static(id="pt-status", classes="pt-status")
            with Vertical(id="pt-table-wrapper"):
                yield DataTable(id="pt-table")

    def on_mount(self):
        table = self.query_one("#pt-table", DataTable)
        table.add_columns("PID", "Name", "CPU%", "MEM%", "Status", "Threads", "User")
        table.cursor_type = "row"
        table.zebra_stripes = True
        self._timer = self.set_interval(2.0, self._refresh)
        self._refresh()

    def _render_status_bar(self) -> str:
        """Render status bar with sort indicator."""
        arrow = " ▼" if self._descending else " ▲"
        col_display = self._sort_col.upper()
        return (
            f"sort: [{_pct_color(50)}]{col_display}{arrow}[/]  |  "
            f"rows: {len(self._rows)}  |  "
            f"pid: {self._selected_pid_text()}"
        )

    def _selected_pid_text(self) -> str:
        pid = self._selected_pid()
        if pid is None:
            return "none"
        return str(pid)

    def _refresh(self):
        query = self.query_one("#pt-filter", Input).value
        table = self.query_one("#pt-table", DataTable)
        table.clear()

        rows = self.svc.list_processes(
            include_kernel=False,
            sort_by=self._sort_col,
            descending=self._descending,
            query=query,
            limit=150,
        )
        self._rows = rows
        for r in rows:
            cpu_color = _pct_color(r.cpu_percent)
            mem_color = _pct_color(r.memory_percent)
            table.add_row(
                str(r.pid),
                r.name,
                Text(f"{r.cpu_percent:.1f}", style=cpu_color),
                Text(f"{r.memory_percent:.1f}", style=mem_color),
                r.status[:3],
                str(r.threads),
                r.username or "",
            )
        self._update_meta()

    def _selected_pid(self) -> int | None:
        table = self.query_one("#pt-table", DataTable)
        row_key = table.cursor_row
        if row_key is None or not table.row_count:
            return None
        return int(table.get_row_at(row_key)[0])

    def _selected_row(self):
        pid = self._selected_pid()
        if pid is None:
            return None
        for row in self._rows:
            if row.pid == pid:
                return row
        return None

    def _update_meta(self) -> None:
        selected = self._selected_row()
        self.query_one("#pt-detail", Static).update(format_process_detail(selected))
        self.query_one("#pt-status", Static).update(self._render_status_bar())

    def action_kill(self):
        pid = self._selected_pid()
        if pid is None:
            return
        try:
            result = self.svc.terminate_process(pid)
            self.post_message(self.ProcessSelected(pid, str(pid), "kill"))
            self.notify(result["message"], severity="information")
            self._refresh()
        except Exception as e:
            self.notify(str(e), severity="error")

    def action_stop(self):
        pid = self._selected_pid()
        if pid is None:
            return
        try:
            result = self.svc.suspend_process(pid)
            self.post_message(self.ProcessSelected(pid, str(pid), "stop"))
            self.notify(result["message"], severity="information")
            self._refresh()
        except Exception as e:
            self.notify(str(e), severity="error")

    def action_resume(self):
        pid = self._selected_pid()
        if pid is None:
            return
        try:
            result = self.svc.resume_process(pid)
            self.post_message(self.ProcessSelected(pid, str(pid), "resume"))
            self.notify(result["message"], severity="information")
            self._refresh()
        except Exception as e:
            self.notify(str(e), severity="error")

    def action_toggle_sort(self):
        self._sort_col = next_sort_column(self._sort_col)
        self.notify(f"Sort: {self._sort_col} ({'desc' if self._descending else 'asc'})", severity="information")
        self._refresh()

    def action_toggle_sort_direction(self):
        self._descending = not self._descending
        self.notify(f"Direction: {'desc' if self._descending else 'asc'}", severity="information")
        self._refresh()

    def on_input_changed(self, event: Input.Changed):
        self._refresh()

    def on_data_table_row_highlighted(self, event: DataTable.RowHighlighted):
        self._update_meta()

    def apply_theme(self, theme: str) -> None:
        is_light = theme == "light"
        if is_light:
            self.add_class("vanta-light")
        else:
            self.remove_class("vanta-light")

    CSS = """
    ProcessTable {
        height: 1fr;
    }
    .pt-toolbar {
        height: 3;
        margin-bottom: 1;
    }
    .pt-input {
        width: 50%;
    }
    .pt-hint {
        width: 1fr;
        padding: 1 0 0 2;
    }
    .pt-detail {
        height: 3;
        border: solid #1e1e3f;
        background: #0f0f1a;
        color: #cbd5e1;
        padding: 1;
        margin-bottom: 1;
    }
    .pt-status {
        height: 1;
        color: #64748b;
        margin: 0 1 1 1;
    }
    #pt-table-wrapper {
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

    .vanta-light .pt-detail {
        border: solid #d1d5db;
        background: #ffffff;
        color: #1a1a1a;
    }
    .vanta-light .pt-status {
        color: #6b7280;
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
