"""Tests for config-driven dashboard widget behavior."""

import asyncio
from pathlib import Path

from monitor.core.dashboard_config import load_dashboard_config
from monitor.core.dashboard_widgets import (
    WidgetRenderCache,
    build_calendar_widget,
    build_clock_widget,
    build_custom_text_widget,
    build_image_widget,
    build_matrix_widget,
    build_music_widget,
    build_wallpaper_widget,
    build_yazi_widget,
    paginate_widgets,
    widget_refresh_interval,
)


def test_load_dashboard_config_returns_enabled_widget_order() -> None:
    config = load_dashboard_config(Path("/home/zius/Projects/vanta/monitor/config.json"))
    assert config.ui.refresh_rate == 0.5
    assert config.process.max_display == 15
    assert config.enabled_widget_names() == [
        "dashboard",
        "clock",
        "calendar",
        "matrix",
        "music_viz",
        "pstree",
        "fastfetch",
        "custom_text",
        "yazi",
        "process_manager",
        "system_stats",
    ]


def test_dashboard_page_size_and_history_rules() -> None:
    config = load_dashboard_config(Path("/home/zius/Projects/vanta/monitor/config.json"))
    assert config.page_size_for_width(170) == 4
    assert config.page_size_for_width(120) == 3
    assert config.page_size_for_width(90) == 2
    assert config.page_size_for_width(60) == 1
    assert config.show_history_for_height(40) is True
    assert config.show_history_for_height(30) is False
    assert config.compact_mode_for_height(25) is True
    assert config.ultra_compact_mode_for_height(19) is True


def test_clock_widget_renders_24h_time() -> None:
    text = build_clock_widget({"format": "24h", "show_date": True}, now_text="2026-06-19 22:41:03")
    assert "22:41:03" in text
    assert "2026-06-19" in text


def test_calendar_widget_contains_month_heading() -> None:
    text = build_calendar_widget({}, year=2026, month=6)
    assert "June 2026" in text
    assert "Mo Tu We Th Fr Sa Su" in text


def test_custom_text_widget_joins_sections() -> None:
    text = build_custom_text_widget(
        {
            "sections": [
                {"title": "Status", "content": "alpha\nbeta"},
                {"title": "Links", "content": "github.com/ziuus/vanta"},
            ]
        }
    )
    assert "Status" in text
    assert "alpha" in text
    assert "Links" in text


def test_matrix_widget_respects_dimensions() -> None:
    import re
    text = build_matrix_widget({"density": 1.0}, width=12, height=4, tick=7)
    lines = text.splitlines()
    assert len(lines) == 4
    # Strip Rich markup tags for length check
    plain = [re.sub(r"\[/?[^\]]*\]", "", l) for l in lines]
    assert all(len(l) == 12 for l in plain), f"line lengths: {[len(l) for l in plain]}"


def test_music_widget_renders_bar_rows() -> None:
    text = build_music_widget({"bars": 8, "sensitivity": 0.5}, tick=3)
    assert any(bar in text for bar in "▁▂▃▄▅▆▇█")


def test_yazi_widget_lists_directory_entries(tmp_path: Path) -> None:
    (tmp_path / "a.txt").write_text("a")
    (tmp_path / "b").mkdir()
    text = build_yazi_widget({"cwd": str(tmp_path)})
    assert str(tmp_path) in text
    assert "a.txt" in text
    assert "b/" in text


def test_wallpaper_widget_reports_missing_dir(tmp_path: Path) -> None:
    text = build_wallpaper_widget({"directory": str(tmp_path / 'missing'), "interval": 300})
    assert "not found" in text.lower()


def test_image_widget_reports_missing_image() -> None:
    text = build_image_widget({"path": "/no/such/file.png"})
    assert "missing" in text.lower()


def test_paginate_widgets_groups_items_into_pages() -> None:
    pages = paginate_widgets(["clock", "matrix", "calendar", "yazi", "pstree"], page_size=3)
    assert pages == [["clock", "matrix", "calendar"], ["yazi", "pstree"]]


def test_widget_refresh_interval_prefers_config_value() -> None:
    assert widget_refresh_interval("fastfetch", {"refresh_interval": 77}) == 77
    assert widget_refresh_interval("wallpaper", {"interval": 300}) == 300
    assert widget_refresh_interval("clock", {}) == 1


def test_widget_render_cache_reuses_content_before_ttl() -> None:
    cache = WidgetRenderCache()
    cfg = {"refresh_interval": 60, "cwd": "/tmp"}
    first = cache.render("yazi", cfg, now=100.0)
    second = cache.render("yazi", cfg, now=120.0)
    assert first == second


def test_default_theme_is_light() -> None:
    config = load_dashboard_config(Path("/home/zius/Projects/vanta/monitor/config.json"))
    assert config.ui.theme == "light"


def test_dark_theme_config_override() -> None:
    config = load_dashboard_config(Path("/home/zius/Projects/vanta/monitor/config.json"))
    config.ui.theme = "dark"
    assert config.ui.theme == "dark"


def test_theme_css_class_toggle() -> None:
    """apply_theme on OverviewScreen adds/removes vanta-light class."""
    from textual.app import App
    from monitor.screens.overview import OverviewScreen

    class TestHarness(App):
        def compose(self):
            yield OverviewScreen()

    async def scenario() -> None:
        app = TestHarness()
        async with app.run_test():
            screen = app.query_one(OverviewScreen)
            # no theme applied yet in test context
            assert "vanta-light" not in screen.classes
            # apply light
            screen.apply_theme("light")
            assert "vanta-light" in screen.classes
            # toggle to dark
            screen.apply_theme("dark")
            assert "vanta-light" not in screen.classes
            # toggle back
            screen.apply_theme("light")
            assert "vanta-light" in screen.classes

    asyncio.run(scenario())


def test_app_toggles_theme_through_action() -> None:
    """VantaMonitorTUI's toggle_theme action swaps theme."""
    from monitor.app import VantaMonitorTUI

    async def scenario() -> None:
        app = VantaMonitorTUI()
        async with app.run_test(size=(120, 40)) as pilot:
            assert app._theme == "light", f"expected light got {app._theme}"
            await pilot.press("T")
            assert app._theme == "dark"
            await pilot.press("T")
            assert app._theme == "light"

    asyncio.run(scenario())


def test_app_cycles_theme_presets() -> None:
    """Shift+P / P should move through the named palette presets."""
    from monitor.app import VantaMonitorTUI

    async def scenario() -> None:
        app = VantaMonitorTUI()
        async with app.run_test(size=(120, 40)) as pilot:
            assert app._theme == "light"
            await pilot.press("P")
            assert app._theme == "dark"
            await pilot.press("P")
            assert app._theme == "monokai"

    asyncio.run(scenario())
