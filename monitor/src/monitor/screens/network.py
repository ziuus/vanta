from textual.app import ComposeResult
from textual.screen import Screen
from textual.widgets import Header, Footer, Static
from textual.containers import Vertical, Horizontal
from textual.binding import Binding

from monitor.core.theme import DARK, LIGHT
from monitor.core.collectors import SystemCollector


class NetworkScreen(Screen):
    """Network interface throughput overview."""

    BINDINGS = [
        Binding("r", "refresh", "Refresh"),
    ]

    def __init__(self):
        super().__init__()
        self.collector = SystemCollector()
        self._refresh_timer = None
        self._theme_name = "light"

    @property
    def pal(self) -> dict[str, str]:
        return LIGHT if self._theme_name == "light" else DARK

    def compose(self) -> ComposeResult:
        yield Header(show_clock=True)
        with Vertical(id="net-body"):
            with Horizontal(id="net-cards"):
                yield Static(id="net-up", classes="net-card")
                yield Static(id="net-down", classes="net-card")
                yield Static(id="net-total", classes="net-card")
        yield Footer()

    def on_mount(self):
        self._refresh_timer = self.set_interval(1.0, self._refresh)
        self._refresh()

    def _refresh(self):
        p = self.pal
        try:
            snap = self.collector.sample()
        except Exception:
            self.query_one("#net-up", Static).update(f"[{p['red']}]collector error[/]")
            return

        def fmt_speed(bps):
            if bps > 1e9:
                return f"[{p['green']}]{bps/1e9:.2f} GB/s[/]"
            if bps > 1e6:
                return f"[{p['green']}]{bps/1e6:.2f} MB/s[/]"
            return f"[{p['green']}]{bps/1e3:.0f} KB/s[/]"

        def fmt_bytes(b):
            if b > 1e12:
                return f"{b/1e12:.2f} TB"
            if b > 1e9:
                return f"{b/1e9:.2f} GB"
            return f"{b/1e6:.1f} MB"

        up_block = f"[{p['text_muted']}]Upload[/]\n{fmt_speed(snap.network.upload_bps)}"
        down_block = f"[{p['text_muted']}]Download[/]\n{fmt_speed(snap.network.download_bps)}"
        total_block = (
            f"[{p['text_muted']}]Cumulative[/]\n"
            f"[{p['text_dim']}]Sent: {fmt_bytes(snap.network.bytes_sent)}[/]\n"
            f"[{p['text_dim']}]Recv: {fmt_bytes(snap.network.bytes_recv)}[/]"
        )

        self.query_one("#net-up").update(up_block)
        self.query_one("#net-down").update(down_block)
        self.query_one("#net-total").update(total_block)

    def action_refresh(self):
        self._refresh()

    def apply_theme(self, theme: str) -> None:
        self._theme_name = theme
        is_light = theme == "light"
        if is_light:
            self.add_class("vanta-light")
        else:
            self.remove_class("vanta-light")
        self._refresh()

    CSS = """
    #net-body {
        padding: 1 2;
    }
    #net-cards {
        height: 6;
    }
    .net-card {
        width: 1fr;
        border: solid #1e1e3f;
        padding: 1;
        background: #0f0f1a;
    }
    
    .vanta-light .net-card {
        border: solid #d1d5db;
        background: #ffffff;
    }
    """
