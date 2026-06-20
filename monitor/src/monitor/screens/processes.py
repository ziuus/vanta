from textual.app import ComposeResult
from textual.screen import Screen
from textual.widgets import Header, Footer
from textual.binding import Binding

from monitor.components.process_table import ProcessTable


class ProcessesScreen(Screen):
    """Dedicated process management screen with kill/stop/resume."""

    BINDINGS = [
        Binding("k", "focused.action_kill()", "Kill", show=True),
        Binding("s", "focused.action_stop()", "Stop", show=True),
        Binding("r", "focused.action_resume()", "Resume", show=True),
        Binding("t", "focused.action_toggle_sort()", "Next sort", show=True),
        Binding("ctrl+t", "focused.action_toggle_sort_direction()", "Dir", show=True),
    ]

    def compose(self) -> ComposeResult:
        yield Header(show_clock=True)
        yield ProcessTable(id="process-table")
        yield Footer()

    CSS = """
    #process-table {
        padding: 0 1;
    }
    """
