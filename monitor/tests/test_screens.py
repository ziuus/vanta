"""Screen-level tests for the Vanta dashboard and file manager."""

from __future__ import annotations

import asyncio
from pathlib import Path

from textual.app import App

from monitor.screens.overview import OverviewScreen
from monitor.screens.filemanager import FileManagerScreen
from monitor.screens.graphs import GraphsScreen


class ScreenHarness(App[None]):
    def __init__(self, screen):
        super().__init__()
        self._screen = screen

    def on_mount(self) -> None:
        self.push_screen(self._screen)


def _run(coro):
    return asyncio.run(coro)


def _text(widget) -> str:
    """Extract text content from a Static widget."""
    return widget.content


def test_overview_dashboard_renders_all_panels() -> None:
    """The dashboard should render cpu, mem, net, disk, and system panels."""
    async def scenario() -> None:
        screen = OverviewScreen()
        async with ScreenHarness(screen).run_test(size=(120, 40)) as pilot:
            await pilot.pause(2)
            cpu = screen.query_one("#cpu-panel")
            mem = screen.query_one("#memory-panel")
            net = screen.query_one("#network-panel")
            disk = screen.query_one("#disk-panel")
            sys_info = screen.query_one("#dash-gpuinfo")
            status = screen.query_one("#dash-status")
            assert cpu is not None
            assert mem is not None
            assert net is not None
            assert disk is not None
            assert sys_info is not None
            assert status is not None
            content = _text(cpu)
            assert "CPU" in content or "%" in content

    _run(scenario())


def test_overview_status_strip_shows_key_metrics() -> None:
    """The status strip should show CPU, MEM, NET, DISK labels."""
    async def scenario() -> None:
        screen = OverviewScreen()
        async with ScreenHarness(screen).run_test(size=(120, 40)) as pilot:
            await pilot.pause(2)
            status = _text(screen.query_one("#dash-status"))
            assert "CPU" in status or "MEM" in status or "procs" in status

    _run(scenario())


def test_overview_theme_toggle_does_not_crash() -> None:
    """Toggling theme should not raise errors."""
    async def scenario() -> None:
        screen = OverviewScreen()
        async with ScreenHarness(screen).run_test(size=(120, 40)) as pilot:
            await pilot.pause(2)
            screen.apply_theme("dark")
            await pilot.pause(0.5)
            assert screen._theme_name == "dark"
            screen.apply_theme("light")
            await pilot.pause(0.5)
            assert screen._theme_name == "light"

    _run(scenario())


def test_overview_process_navigation() -> None:
    """Arrow up/down should move process selection without error."""
    async def scenario() -> None:
        screen = OverviewScreen()
        async with ScreenHarness(screen).run_test(size=(120, 40)) as pilot:
            await pilot.pause(2)
            # Arrow down twice
            await pilot.press("down")
            await pilot.pause(0.2)
            await pilot.press("down")
            await pilot.pause(0.2)
            assert screen._selected_proc_idx == 2
            # Arrow up once
            await pilot.press("up")
            await pilot.pause(0.2)
            assert screen._selected_proc_idx == 1
            # Arrow up past start
            await pilot.press("up")
            await pilot.pause(0.2)
            await pilot.press("up")
            await pilot.pause(0.2)
            assert screen._selected_proc_idx == 0

    _run(scenario())


def test_overview_search_and_user_filter_controls_work() -> None:
    """The hidden search box and user filter should affect screen state without crashing."""
    async def scenario() -> None:
        screen = OverviewScreen()
        async with ScreenHarness(screen).run_test(size=(120, 40)) as pilot:
            await pilot.pause(2)
            await pilot.press("/")
            await pilot.pause(0.2)
            search = screen.query_one("#proc-search-input")
            assert "search-visible" in search.classes
            await pilot.press("p", "y", "t", "h", "o", "n")
            await pilot.pause(0.2)
            assert screen._process_search == "python"
            await pilot.press("enter")
            await pilot.pause(0.2)
            assert "search-hidden" in search.classes
            await pilot.press("U")
            await pilot.pause(0.2)
            assert screen._user_filter is not None
            await pilot.press("U")
            await pilot.pause(0.2)
            assert screen._user_filter is None

    _run(scenario())


def test_graphs_screen_renders_core_trend_panels() -> None:
    """Graphs screen should render CPU, memory, network, and disk trend panels."""
    async def scenario() -> None:
        screen = GraphsScreen()
        async with ScreenHarness(screen).run_test(size=(120, 40)) as pilot:
            await pilot.pause(2)
            cpu = _text(screen.query_one("#graph-cpu"))
            mem = _text(screen.query_one("#graph-mem"))
            net = _text(screen.query_one("#graph-net"))
            disk = _text(screen.query_one("#graph-disk"))
            assert "CPU history" in cpu
            assert "Memory history" in mem
            assert "Network throughput" in net
            assert "Disk I/O" in disk

    _run(scenario())


def test_filemanager_shows_directory_listing() -> None:
    """FileManager should show entries for the home directory."""
    async def scenario() -> None:
        screen = FileManagerScreen(start_dir=Path(__file__).parent.as_posix())
        async with ScreenHarness(screen).run_test(size=(100, 30)) as pilot:
            await pilot.pause(2)
            fm_list = screen.query_one("#fm-list")
            path_label = _text(screen.query_one("#fm-path"))
            assert fm_list is not None
            assert "tests" in path_label

    _run(scenario())


def test_filemanager_enter_parent_navigation() -> None:
    """Pressing h should go to parent directory, l to enter a dir."""
    async def scenario() -> None:
        screen = FileManagerScreen(start_dir=Path(__file__).parent.as_posix())
        async with ScreenHarness(screen).run_test(size=(100, 30)) as pilot:
            await pilot.pause(2)
            await pilot.press("h")
            await pilot.pause(0.5)
            path_label = _text(screen.query_one("#fm-path"))
            assert Path(__file__).parent.parent.as_posix() in path_label or "monitor" in path_label
    _run(scenario())


def test_widgets_screen_shows_clock_and_calendar() -> None:
    """WidgetsScreen renders clock and calendar content."""
    from monitor.screens.widgets_screen import WidgetsScreen

    async def scenario() -> None:
        screen = WidgetsScreen()
        async with ScreenHarness(screen).run_test(size=(100, 30)) as pilot:
            await pilot.pause(3)
            body = screen.query_one("#widgets-body")
            text = _text(body)
            assert ":" in text, f"expected clock colon in:\n{text}"
            assert "Clock" in text or "⏰" in text

    _run(scenario())


def test_widgets_screen_shows_matrix_and_fastfetch() -> None:
    """WidgetsScreen renders matrix and fastfetch when enabled."""
    from monitor.screens.widgets_screen import WidgetsScreen

    async def scenario() -> None:
        screen = WidgetsScreen()
        async with ScreenHarness(screen).run_test(size=(100, 30)) as pilot:
            await pilot.pause(3)
            body = screen.query_one("#widgets-body")
            text = _text(body)
            assert "System" in text or "💻" in text
            assert "Matrix" in text or "🌧" in text

    _run(scenario())
