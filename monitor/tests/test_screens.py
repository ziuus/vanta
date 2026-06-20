"""Screen-level tests for overview, storage, network, and graphs surfaces."""

from __future__ import annotations

import asyncio
from pathlib import Path

from textual.app import App
from textual.containers import Horizontal
from textual.widgets import DataTable, Static, Sparkline

from monitor.core.dashboard_config import load_dashboard_config
from monitor.core.dashboard_widgets import paginate_widgets
from monitor.core.models import (
    CpuSnapshot,
    DiskSnapshot,
    MemorySnapshot,
    NetworkSnapshot,
    ProcessRow,
    SystemSnapshot,
)
from monitor.screens.graphs import GraphScreen
from monitor.screens.network import NetworkScreen
from monitor.screens.overview import OverviewScreen
from monitor.screens.storage import StorageScreen


def make_snapshot() -> SystemSnapshot:
    return SystemSnapshot(
        cpu=CpuSnapshot(
            total_percent=42.5,
            per_core_percent=[35.0, 50.0, 44.0, 41.0],
            load_avg_1m=1.73,
            frequency_mhz=4200.0,
            core_count=4,
        ),
        memory=MemorySnapshot(
            percent=68.2,
            used_bytes=12_000_000_000,
            total_bytes=16_000_000_000,
            available_bytes=4_000_000_000,
            swap_percent=5.5,
        ),
        disks=[
            DiskSnapshot(
                mountpoint="/",
                percent=71.3,
                used_bytes=71_300_000_000,
                free_bytes=28_700_000_000,
                total_bytes=100_000_000_000,
            ),
            DiskSnapshot(
                mountpoint="/home",
                percent=54.2,
                used_bytes=108_400_000_000,
                free_bytes=91_600_000_000,
                total_bytes=200_000_000_000,
            ),
        ],
        network=NetworkSnapshot(
            upload_bps=2_500_000,
            download_bps=12_500_000,
            bytes_sent=12_000_000_000,
            bytes_recv=34_000_000_000,
        ),
        gpu=None,
        process_count=213,
        thread_count=901,
        uptime_seconds=86400.0,
        temperature_c=63.5,
    )


class StaticCollector:
    def __init__(self, snapshot: SystemSnapshot):
        self._snapshot = snapshot

    def sample(self) -> SystemSnapshot:
        return self._snapshot


class FailingCollector:
    def sample(self) -> SystemSnapshot:
        raise RuntimeError("boom")


class StubProcessService:
    def list_processes(self, **_: object) -> list[ProcessRow]:
        return [
            ProcessRow(4242, "python", 32.5, 4.1, "running", 8, "zius"),
            ProcessRow(7777, "node", 20.0, 2.9, "sleeping", 5, "zius"),
        ]


class ScreenHarness(App[None]):
    def __init__(self, screen):
        super().__init__()
        self._screen = screen

    def on_mount(self) -> None:
        self.push_screen(self._screen)


def _run(coro):
    return asyncio.run(coro)


def _render_text(widget: Static) -> str:
    return str(widget.content)


def _prepare_overview(screen: OverviewScreen) -> None:
    screen.collector = StaticCollector(make_snapshot())  # type: ignore[assignment]
    screen.process_service = StubProcessService()  # type: ignore[assignment]
    screen.dashboard_config = load_dashboard_config(Path("/home/zius/Projects/vanta/monitor/config.json"))
    screen._widget_pages = paginate_widgets(screen.dashboard_config.enabled_extra_widget_names(), page_size=3)
    screen._reload_dashboard_config = lambda: None  # type: ignore[method-assign]


def test_overview_screen_shows_status_and_widget_dock() -> None:
    async def scenario() -> None:
        screen = OverviewScreen()
        _prepare_overview(screen)
        async with ScreenHarness(screen).run_test(size=(120, 40)) as pilot:
            await pilot.pause()
            status = _render_text(screen.query_one("#dashboard-status", Static))
            widget_title = _render_text(screen.query_one("#widget-tray-title", Static))
            widget_slot = _render_text(screen.query_one("#widget-slot-0", Static))
            assert "mode=full" in status
            assert "widgets=" in status
            assert "Widget dock" in widget_title
            assert "Clock" in widget_slot or "Calendar" in widget_slot or "Matrix" in widget_slot

    _run(scenario())


def test_overview_screen_compacts_on_small_terminal() -> None:
    async def scenario() -> None:
        screen = OverviewScreen()
        _prepare_overview(screen)
        async with ScreenHarness(screen).run_test(size=(90, 28)) as pilot:
            await pilot.pause()
            status = _render_text(screen.query_one("#dashboard-status", Static))
            assert "mode=compact" in status
            assert screen.query_one("#widget-slot-0", Static).display is True
            assert screen.query_one("#widget-slot-1", Static).display is True
            assert screen.query_one("#widget-slot-2", Static).display is False
            assert screen.query_one("#history-row", Horizontal).display is False

    _run(scenario())


def test_overview_screen_enters_ultra_compact_mode() -> None:
    async def scenario() -> None:
        screen = OverviewScreen()
        _prepare_overview(screen)
        async with ScreenHarness(screen).run_test(size=(90, 18)) as pilot:
            await pilot.pause()
            status = _render_text(screen.query_one("#dashboard-status", Static))
            assert "mode=tiny" in status
            assert screen.query_one("#widget-tray", Horizontal).display is False
            assert screen.query_one("#widget-tray-title", Static).display is False
            assert screen.query_one("#history-row", Horizontal).display is False

    _run(scenario())


def test_storage_screen_renders_disk_rows() -> None:
    async def scenario() -> None:
        screen = StorageScreen()
        screen.collector = StaticCollector(make_snapshot())  # type: ignore[assignment]
        async with ScreenHarness(screen).run_test() as pilot:
            await pilot.pause()
            table = screen.query_one("#storage-table", DataTable)
            assert table.row_count == 2
            assert table.get_row_at(0)[0] == "/"
            assert table.get_row_at(1)[0] == "/home"
            assert table.get_row_at(0)[4] == "71.3%"

    _run(scenario())


def test_network_screen_renders_live_blocks() -> None:
    async def scenario() -> None:
        screen = NetworkScreen()
        screen.collector = StaticCollector(make_snapshot())  # type: ignore[assignment]
        async with ScreenHarness(screen).run_test() as pilot:
            await pilot.pause()
            up = _render_text(screen.query_one("#net-up", Static))
            down = _render_text(screen.query_one("#net-down", Static))
            total = _render_text(screen.query_one("#net-total", Static))
            assert "Upload" in up
            assert "2.50 MB/s" in up
            assert "Download" in down
            assert "12.50 MB/s" in down
            assert "Cumulative" in total
            assert "12.00 GB" in total

    _run(scenario())


def test_network_screen_shows_collector_error() -> None:
    async def scenario() -> None:
        screen = NetworkScreen()
        screen.collector = FailingCollector()  # type: ignore[assignment]
        async with ScreenHarness(screen).run_test() as pilot:
            await pilot.pause()
            up = _render_text(screen.query_one("#net-up", Static))
            assert "collector error" in up.lower()

    _run(scenario())


def test_graph_screen_populates_titles_and_sparklines() -> None:
    async def scenario() -> None:
        screen = GraphScreen()
        screen.collector = StaticCollector(make_snapshot())  # type: ignore[assignment]
        async with ScreenHarness(screen).run_test(size=(120, 40)) as pilot:
            await pilot.pause()
            cpu_title = _render_text(screen.query_one("#gr-cpu-title", Static))
            net_title = _render_text(screen.query_one("#gr-net-title", Static))
            system_info = _render_text(screen.query_one("#gr-system-info", Static))
            cpu_long = screen.query_one("#gr-cpu-long", Sparkline)
            disk_short = screen.query_one("#gr-disk-short", Sparkline)
            assert "CPU" in cpu_title
            assert "42.5" in cpu_title
            assert "Net up" in net_title
            assert "Net down" in net_title
            assert "Load: 1.73" in system_info
            assert "Temp: 63.5°C" in system_info
            assert cpu_long.data and cpu_long.data[-1] == 42.5
            assert disk_short.data and disk_short.data[-1] == 71.3

    _run(scenario())


def test_graph_screen_handles_missing_temperature() -> None:
    async def scenario() -> None:
        snap = make_snapshot()
        snap.temperature_c = None
        screen = GraphScreen()
        screen.collector = StaticCollector(snap)  # type: ignore[assignment]
        async with ScreenHarness(screen).run_test(size=(120, 40)) as pilot:
            await pilot.pause()
            system_info = _render_text(screen.query_one("#gr-system-info", Static))
            assert "Load: 1.73" in system_info
            assert "Procs: 213" in system_info
            assert "Temp:" not in system_info

    _run(scenario())
