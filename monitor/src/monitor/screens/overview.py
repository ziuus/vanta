from __future__ import annotations

import getpass
from datetime import datetime
from pathlib import Path

from textual.app import ComposeResult
from textual.binding import Binding
from textual.containers import Horizontal, Vertical
from textual.events import Click, Resize
from textual.screen import Screen
from textual.widgets import Footer, Header, Input, ListItem, ListView, Static

from monitor.core.collectors import SystemCollector
from monitor.core.dashboard_config import load_dashboard_config
from monitor.core.history import HistoryBuffer
from monitor.core.media import MediaController
from monitor.core.models import ProcessRow
from monitor.core.overview_presenter import _bar_color, _nowplaying_text, format_rate_binary, make_overview_panels
from monitor.core.process_presenter import format_process_detail
from monitor.core.process_service import ProcessService, SIGNAL_LIST, next_sort_column, prev_sort_column
from monitor.core.theme import get_palette, is_light_theme, theme_label

CONFIG_PATH = Path(__file__).resolve().parents[3] / "config.json"
STYLES_DIR = Path(__file__).resolve().parent.parent / "styles"
BARS = "▁▂▃▄▅▆▇█"


def _signal_desc(sig: str) -> str:
    descs = {
        "TERM": "Graceful termination",
        "KILL": "Force kill",
        "STOP": "Suspend (pause)",
        "CONT": "Resume (continue)",
        "HUP": "Hangup / reload config",
        "INT": "Interrupt (Ctrl+C)",
        "USR1": "User-defined 1",
        "USR2": "User-defined 2",
    }
    return descs.get(sig, "")


class SignalMenu(Screen):
    BINDINGS = [Binding("escape", "dismiss", "Close")]

    def __init__(self, pid: int, name: str, pal: dict[str, str]):
        super().__init__()
        self._pid = pid
        self._name = name
        self._pal = pal

    def compose(self) -> ComposeResult:
        p = self._pal
        yield Static(f"[{p['accent']}]Select signal for PID {self._pid} ({self._name})[/]", id="sm-title")
        with ListView(id="sm-list"):
            for sig in SIGNAL_LIST:
                yield ListItem(Static(f"{sig:<8}  {_signal_desc(sig)}"))

    def on_list_view_selected(self, event: ListView.Selected) -> None:
        label = event.item.children[0].content if event.item.children else "TERM"
        self.dismiss(label.split()[0])

    CSS = """
    SignalMenu {
        align: center middle;
        background: rgba(10, 10, 15, 0.85);
    }
    #sm-title {
        padding: 0 0 1 0;
        text-align: center;
    }
    #sm-list {
        width: 48;
        height: auto;
        border: solid #06b6d4;
        background: #0f0f1a;
    }
    #sm-list > ListItem {
        padding: 0 2;
    }
    """


class ProcessDetail(Screen):
    BINDINGS = [Binding("escape", "dismiss", "Close"), Binding("q", "dismiss", "Close")]

    def __init__(self, pid: int, detail: dict, pal: dict[str, str]):
        super().__init__()
        self._pid = pid
        self._detail = detail
        self._pal = pal

    def compose(self) -> ComposeResult:
        p = self._pal
        d = self._detail
        lines = [f"[{p['accent']}]Process Detail — PID {d.get('pid', self._pid)}[/]", ""]
        ordered_fields = [
            ("name", "Name"),
            ("exe", "Executable"),
            ("cmdline", "Command"),
            ("cwd", "CWD"),
            ("username", "User"),
            ("status", "Status"),
            ("nice", "Nice"),
            ("cpu_percent", "CPU%"),
            ("memory_percent", "MEM%"),
            ("memory_rss", "RSS"),
            ("memory_vms", "VMS"),
            ("threads", "Threads"),
            ("children", "Children"),
            ("fds", "File Descriptors"),
            ("connections", "Connections"),
            ("cpu_affinity", "CPU Affinity"),
            ("create_time", "Started"),
        ]
        for key, label in ordered_fields:
            value = d.get(key)
            if value in (None, "", []):
                continue
            if key in {"memory_rss", "memory_vms"} and isinstance(value, (int, float)):
                value = f"{value / (1024 ** 2):.0f} MiB"
            elif key == "create_time" and isinstance(value, (int, float)):
                value = datetime.fromtimestamp(value).strftime("%Y-%m-%d %H:%M:%S")
            elif key == "cpu_affinity" and isinstance(value, list):
                value = ", ".join(str(core) for core in value)
            elif isinstance(value, float):
                value = f"{value:.1f}"
            lines.append(f"  [{p['accent']}]{label:<17}[/] [{p['text']}]{value}[/]")
        env_lines = d.get("environment_preview") or []
        if env_lines:
            lines.extend(["", f"[{p['accent']}]Environment[/]"])
            lines.extend(f"  [{p['text_muted']}]{line}[/]" for line in env_lines)
        lines.extend(["", f"[{p['text_dim']}]Esc/q to close[/]"])
        yield Static("\n".join(lines), id="pd-content")

    CSS = """
    ProcessDetail {
        align: center middle;
        background: rgba(10, 10, 15, 0.88);
    }
    #pd-content {
        width: 78;
        height: auto;
        border: solid #06b6d4;
        background: #0f0f1a;
        padding: 1 2;
        color: #cbd5e1;
    }
    """


class OverviewScreen(Screen):
    """Dense home dashboard with operator-focused process controls."""

    BINDINGS = [
        Binding("slash", "focus_search", "Search", show=False),
        Binding("escape", "clear_search", "Clear", show=False),
        Binding("j", "proc_down", "Down", show=False),
        Binding("down", "proc_down", "Down", show=False),
        Binding("k", "proc_up", "Up", show=False),
        Binding("up", "proc_up", "Up", show=False),
        Binding("d", "process_detail", "Detail", show=False),
        Binding("K", "signal_menu", "Signal", show=False),
        Binding("c", "cycle_sort", "Sort", show=False),
        Binding("C", "cycle_sort_rev", "Sort Back", show=False),
        Binding("u", "toggle_kernel", "Kernel", show=False),
        Binding("U", "toggle_user_filter", "User", show=False),
        Binding("F8", "toggle_process_view", "Tree", show=False),
        Binding("z", "media_play_pause", "Play/Pause", show=False),
        Binding("x", "media_next", "Next", show=False),
        Binding("c", "media_prev", "Prev", show=False),
    ]

    def __init__(self) -> None:
        super().__init__()
        self.collector = SystemCollector()
        self.process_service = ProcessService()
        self.dashboard_config = load_dashboard_config(CONFIG_PATH)
        self._theme_name = self.dashboard_config.ui.theme or "light"
        self._cpu_hist = HistoryBuffer(size=80)
        self._mem_hist = HistoryBuffer(size=80)
        self._net_up_hist = HistoryBuffer(size=80)
        self._net_down_hist = HistoryBuffer(size=80)
        self._disk_hist = HistoryBuffer(size=80)
        self._last_snap = None
        self._refresh_timer = None
        self._process_count = 0
        self._visible_count = 0
        self._last_error: str | None = None
        self._layout_mode = "full"
        self._sort_col = "cpu"
        self._descending = True
        self._tree_view = False
        self._selected_proc_idx = 0
        self._process_search = ""
        self._show_kernel = self.dashboard_config.process.show_kernel
        self._user_filter: str | None = None
        self._processes: list[ProcessRow] = []
        self._dom_ready = False
        self._media_ctrl = MediaController()

    @property
    def pal(self) -> dict[str, str]:
        return get_palette(self._theme_name)

    def compose(self) -> ComposeResult:
        yield Header(show_clock=True)
        with Vertical(id="dash-body"):
            yield Static(id="dash-status", classes="status-strip")
            with Horizontal(id="dash-top"):
                yield Static(id="cpu-panel", classes="panel left-panel")
                yield Static(id="memory-panel", classes="panel right-panel")
            with Horizontal(id="dash-mid"):
                yield Static(id="network-panel", classes="panel left-panel")
                yield Static(id="disk-panel", classes="panel right-panel")
            yield Static(id="dash-gpuinfo", classes="sys-strip")
            yield Static(id="proc-detail", classes="proc-detail-bar")
            yield Static(id="proc-panel", classes="proc-box")
            yield Input(placeholder="Search process name or PID…", id="proc-search-input", classes="search-hidden")
        yield Footer()

    def on_mount(self) -> None:
        self._dom_ready = True
        search = self.query_one("#proc-search-input", Input)
        search.can_focus = False
        self.app.set_focus(None)
        self._reflow_layout()
        self._refresh_timer = self.set_interval(max(0.5, self.dashboard_config.ui.refresh_rate), self._refresh)
        self._refresh()

    def on_resize(self, _: Resize) -> None:
        self._reflow_layout()
        self._refresh()

    def _reflow_layout(self) -> None:
        height = self.size.height
        compact = self.dashboard_config.compact_mode_for_height(height)
        tiny = self.dashboard_config.ultra_compact_mode_for_height(height)
        self.set_class(compact and not tiny, "compact")
        self.set_class(tiny, "tiny")
        self._layout_mode = "tiny" if tiny else "compact" if compact else "full"

    def _visible_row_limit(self) -> int:
        height = self.size.height or 40
        if height < 20:
            return 7
        if height < 30:
            return 11
        return max(12, min(24, height - 18))

    def _refresh(self) -> None:
        p = self.pal
        try:
            snap = self.collector.sample()
            self._last_snap = snap
            self._last_error = None
        except Exception as exc:
            self._last_error = str(exc)
            if self._dom_ready:
                self._update_status()
            return

        self._cpu_hist.push(snap.cpu.total_percent)
        self._mem_hist.push(snap.memory.percent)
        self._net_up_hist.push(snap.network.upload_bps / (1024 ** 2))
        self._net_down_hist.push(snap.network.download_bps / (1024 ** 2))
        if snap.disks:
            self._disk_hist.push(snap.disks[0].percent)

        self._refresh_processes()
        panels = make_overview_panels(snap, p)
        self.query_one("#cpu-panel", Static).update(panels["cpu"])
        self.query_one("#memory-panel", Static).update(panels["memory"])
        self.query_one("#network-panel", Static).update(panels["network"])
        self.query_one("#disk-panel", Static).update(panels["disks"])
        self.query_one("#dash-gpuinfo", Static).update(_nowplaying_text(snap, p))
        self._render_process_list()
        self._update_detail_bar()
        self._update_status()

    def _refresh_processes(self) -> None:
        max_procs = max(self.dashboard_config.process.max_display, 240)
        self._processes = self.process_service.list_processes(
            include_kernel=self._show_kernel,
            sort_by=self._sort_col,
            descending=self._descending,
            query=self._process_search,
            limit=max_procs,
            username=self._user_filter,
        )
        self._process_count = psutil_process_count()
        self._visible_count = len(self._processes)
        if self._visible_count == 0:
            self._selected_proc_idx = 0
        else:
            self._selected_proc_idx = max(0, min(self._selected_proc_idx, self._visible_count - 1))

    def _process_header(self) -> str:
        p = self.pal
        sort_arrow = "▼" if self._descending else "▲"
        columns = [("PID", "pid"), ("NAME", "name"), ("CPU%", "cpu"), ("MEM%", "memory"), ("THR", "threads"), ("ST", "status")]
        parts = []
        for label, key in columns:
            if key == self._sort_col:
                parts.append(f"[{p['accent']}]{label} {sort_arrow}[/]")
            else:
                parts.append(f"[{p['text_muted']}]{label}[/]")
        return "  ".join(parts)

    def _tree_rows(self) -> list[tuple[int, ProcessRow]]:
        rows = self._processes
        by_parent: dict[int, list[ProcessRow]] = {}
        pid_set = {row.pid for row in rows}
        roots: list[ProcessRow] = []
        for row in rows:
            if row.ppid in pid_set and row.ppid != row.pid:
                by_parent.setdefault(row.ppid, []).append(row)
            else:
                roots.append(row)
        ordered: list[tuple[int, ProcessRow]] = []
        seen: set[int] = set()

        def walk(depth: int, row: ProcessRow) -> None:
            if row.pid in seen or len(ordered) >= len(rows):
                return
            seen.add(row.pid)
            ordered.append((depth, row))
            children = by_parent.get(row.pid, [])
            children.sort(key=lambda child: (-child.cpu_percent, child.name.lower()))
            for child in children:
                walk(depth + 1, child)

        roots.sort(key=lambda row: (-row.cpu_percent, row.name.lower()))
        for root in roots:
            walk(0, root)
        for row in rows:
            if row.pid not in seen:
                ordered.append((0, row))
        return ordered

    def _render_process_list(self) -> None:
        p = self.pal
        header = "  " + self._process_header()
        lines = [header]
        limit = self._visible_row_limit()
        rows = self._tree_rows() if self._tree_view else [(0, row) for row in self._processes]

        for index, (depth, row) in enumerate(rows[:limit]):
            sel = f"[{p['accent']}]▸[/]" if index == self._selected_proc_idx else " "
            cpu_c = _bar_color(row.cpu_percent, p)
            mem_c = _bar_color(row.memory_percent, p)
            prefix = ("  " * min(depth, 4) + "└─") if self._tree_view and depth else ""
            name_width = 24 if not self._tree_view else 20
            name = (prefix + row.name)[:name_width]
            lines.append(
                f"{sel}[{p['text_dim']}]{row.pid:>6}[/]  "
                f"[{p['text']}]{name:<{name_width}}[/]  "
                f"[{cpu_c}]{row.cpu_percent:>5.1f}[/]  "
                f"[{mem_c}]{row.memory_percent:>5.1f}[/]  "
                f"[{p['text_dim']}]{row.threads:>3}[/]  "
                f"[{p['text_muted']}]{row.status[:5]:<5}[/]"
            )

        if not lines[1:]:
            lines.append(f"[{p['text_dim']}]No matching processes[/]")

        lines.extend(["", self._process_footer()])
        self.query_one("#proc-panel", Static).update("\n".join(lines))

    def _process_footer(self) -> str:
        p = self.pal
        arrow = "▼" if self._descending else "▲"
        view = "tree" if self._tree_view else "flat"
        kernel = "on" if self._show_kernel else "off"
        user = self._user_filter or "all"
        search = self._process_search or "none"
        return (
            f"[{p['accent']}]sort[/]=[{p['text']}]{self._sort_col} {arrow}[/]  "
            f"[{p['accent']}]filter[/]=[{p['text']}]{search}[/]  "
            f"[{p['accent']}]kernel[/]=[{p['text']}]{kernel}[/]  "
            f"[{p['accent']}]user[/]=[{p['text']}]{user}[/]  "
            f"[{p['accent']}]view[/]=[{p['text']}]{view}[/]  "
            f"[{p['text_dim']}]j/k move  d detail  K signal  / search[/]"
        )

    def _selected_row(self) -> ProcessRow | None:
        if self._processes and 0 <= self._selected_proc_idx < len(self._processes):
            return self._processes[self._selected_proc_idx]
        return None

    def _update_detail_bar(self) -> None:
        p = self.pal
        row = self._selected_row()
        if row is None:
            self.query_one("#proc-detail", Static).update(f"[{p['text_dim']}]No process selected[/]")
            return
        self.query_one("#proc-detail", Static).update(format_process_detail(row))

    def _update_status(self) -> None:
        p = self.pal
        snap = self._last_snap
        if not snap:
            self.query_one("#dash-status", Static).update(f"[{p['text_dim']}]Waiting for data...[/]")
            return

        def spark(values: list[float], width: int = 8) -> str:
            vals = values[-width:]
            if len(vals) < 2:
                return "─" * width
            lo = min(vals)
            hi = max(vals)
            rng = hi - lo or 1.0
            return "".join(BARS[min(len(BARS) - 1, int((v - lo) / rng * (len(BARS) - 1)))] for v in vals)

        disk_pct = f"DISK {snap.disks[0].percent:.0f}% {spark(self._disk_hist.values())}" if snap.disks else "DISK n/a"
        bat = ""
        if snap.battery:
            icon = "⚡" if snap.battery.status in ("Charging", "Full") else "🔋"
            bat = f"  {icon}{snap.battery.percent:.0f}%"
        power = f"  {snap.cpu_power_watts:.0f}W" if snap.cpu_power_watts > 0 else ""
        uptime_h = snap.uptime_seconds / 3600.0
        err = f"  [{p['red']}]err={self._last_error}[/]" if self._last_error else ""
        self.query_one("#dash-status", Static).update(
            f"[{p['text']}]CPU {snap.cpu.total_percent:.0f}% {spark(self._cpu_hist.values())}  "
            f"MEM {snap.memory.percent:.0f}% {spark(self._mem_hist.values())}  "
            f"NET ↑{format_rate_binary(snap.network.upload_bps).replace(' ', '')} ↓{format_rate_binary(snap.network.download_bps).replace(' ', '')}  "
            f"{disk_pct}{bat}{power}[/]  "
            f"[{p['text_dim']}]Up {uptime_h:.1f}h  procs {self._visible_count}/{self._process_count}  mode={self._layout_mode}  theme={theme_label(self._theme_name)}[/]{err}"
        )

    def action_focus_search(self) -> None:
        inp = self.query_one("#proc-search-input", Input)
        inp.can_focus = True
        inp.remove_class("search-hidden")
        inp.add_class("search-visible")
        inp.focus()
        inp.value = self._process_search

    def action_clear_search(self) -> None:
        inp = self.query_one("#proc-search-input", Input)
        self._process_search = ""
        inp.value = ""
        inp.can_focus = False
        inp.remove_class("search-visible")
        inp.add_class("search-hidden")
        self.app.set_focus(None)
        self._refresh()

    def on_input_changed(self, event: Input.Changed) -> None:
        if event.input.id != "proc-search-input":
            return
        self._process_search = event.value
        self._refresh_processes()
        self._render_process_list()
        self._update_detail_bar()
        self._update_status()

    def on_input_submitted(self, event: Input.Submitted) -> None:
        if event.input.id != "proc-search-input":
            return
        event.input.can_focus = False
        event.input.remove_class("search-visible")
        event.input.add_class("search-hidden")
        self.app.set_focus(None)
        self._refresh()

    def action_proc_down(self) -> None:
        if self._selected_proc_idx < self._visible_count - 1:
            self._selected_proc_idx += 1
            self._render_process_list()
            self._update_detail_bar()

    def action_proc_up(self) -> None:
        if self._selected_proc_idx > 0:
            self._selected_proc_idx -= 1
            self._render_process_list()
            self._update_detail_bar()

    def action_cycle_sort(self) -> None:
        self._sort_col = next_sort_column(self._sort_col)
        self._refresh()

    def action_cycle_sort_rev(self) -> None:
        self._sort_col = prev_sort_column(self._sort_col)
        self._refresh()

    def action_toggle_kernel(self) -> None:
        self._show_kernel = not self._show_kernel
        self._refresh()

    def action_toggle_user_filter(self) -> None:
        self._user_filter = None if self._user_filter else getpass.getuser()
        self._refresh()

    def action_toggle_process_view(self) -> None:
        self._tree_view = not self._tree_view
        self._refresh()

    def action_media_play_pause(self) -> None:
        if self._media_ctrl.available:
            self._media_ctrl.play_pause()
            self._update_status()

    def action_media_next(self) -> None:
        if self._media_ctrl.available:
            self._media_ctrl.next()

    def action_media_prev(self) -> None:
        if self._media_ctrl.available:
            self._media_ctrl.previous()

    def action_signal_menu(self) -> None:
        row = self._selected_row()
        if row is None:
            return

        def on_signal(sig_name: str | None) -> None:
            if not sig_name:
                return
            try:
                result = self.process_service.send_signal(row.pid, sig_name)
                self._last_error = result["message"]
            except Exception as exc:
                self._last_error = f"{sig_name} {row.pid}: {exc}"
            self._refresh()

        self.push_screen(SignalMenu(row.pid, row.name, self.pal), on_signal)

    def action_process_detail(self) -> None:
        row = self._selected_row()
        if row is None:
            return
        detail = self.process_service.get_process_detail(row.pid)
        self.app.push_screen(ProcessDetail(row.pid, detail, self.pal))

    def action_toggle_app_theme(self) -> None:
        if hasattr(self, "app") and self.app:
            self.app.action_toggle_theme()

    def on_click(self, event: Click) -> None:
        if getattr(event.widget, "id", None) != "proc-panel":
            return
        local_y = event.y
        row_idx = local_y - 1
        if 0 <= row_idx < min(self._visible_row_limit(), len(self._processes)):
            self._selected_proc_idx = row_idx
            self._render_process_list()
            self._update_detail_bar()

    def apply_theme(self, theme: str) -> None:
        self._theme_name = theme
        if is_light_theme(theme):
            self.add_class("vanta-light")
        else:
            self.remove_class("vanta-light")
        if self._dom_ready:
            self._refresh()

    CSS_PATH = str(STYLES_DIR / "overview.tcss")


def psutil_process_count() -> int:
    try:
        import psutil
        return len(psutil.pids())
    except Exception:
        return 0
