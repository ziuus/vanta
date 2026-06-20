"""Tests for graph/trend presenter helpers."""

from monitor.core.graph_presenter import (
    auto_scale_range,
    format_graph_header,
    make_graph_label,
    time_range_label,
)


def test_auto_scale_range_returns_sensible_bounds():
    lo, hi = auto_scale_range([10.0, 20.0, 30.0, 25.0, 15.0])
    assert lo <= 10.0
    assert hi >= 30.0
    assert hi > lo


def test_auto_scale_range_handles_flat_data():
    lo, hi = auto_scale_range([42.0, 42.0, 42.0])
    assert lo < 42.0
    assert hi > 42.0


def test_auto_scale_range_handles_empty():
    lo, hi = auto_scale_range([])
    assert lo == 0.0
    assert hi == 100.0


def test_format_graph_header_shows_title_and_value():
    text = format_graph_header("CPU", 67.4, unit="%")
    assert "CPU" in text
    assert "67.4%" in text


def test_format_graph_header_supports_bytes():
    text = format_graph_header("Memory", 73.5, unit="%")
    assert "Memory" in text
    assert "73.5%" in text


def test_make_graph_label_shows_range():
    label = make_graph_label([5.0, 15.0, 25.0])
    assert "5.0" in label
    assert "25.0" in label
    assert "15.0" in label


def test_make_graph_label_supports_bytestyle():
    label = make_graph_label([5 * 1024**2, 50 * 1024**2], bytestyle=True)
    assert "MiB" in label
    assert "50.0" in label
    assert "5.0" in label


def test_time_range_label_formats_correctly():
    assert time_range_label(60) == "1m"
    assert time_range_label(120) == "2m"
    assert time_range_label(600) == "10m"
