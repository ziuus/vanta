from monitor.core.models import ProcessRow
from monitor.core.process_presenter import (
    format_process_detail,
    format_process_status,
)


def sample_row() -> ProcessRow:
    return ProcessRow(
        pid=4242,
        name="python-long-worker",
        cpu_percent=72.4,
        memory_percent=11.3,
        status="running",
        threads=8,
        username="noel",
    )


def test_format_process_status_shows_sort_direction_filter_and_count():
    text = format_process_status(
        sort_col="memory",
        descending=False,
        query="py",
        total_rows=38,
        selected_pid=4242,
    )

    assert "sort: memory asc" in text
    assert "filter: py" in text
    assert "rows: 38" in text
    assert "pid: 4242" in text


def test_format_process_status_handles_empty_filter_and_no_selection():
    text = format_process_status(
        sort_col="cpu",
        descending=True,
        query="",
        total_rows=5,
        selected_pid=None,
    )

    assert "sort: cpu desc" in text
    assert "filter: none" in text
    assert "pid: none" in text


def test_format_process_detail_shows_dense_selected_row_summary():
    text = format_process_detail(sample_row())

    assert "python-long-worker" in text
    assert "PID 4242" in text
    assert "72.4%" in text
    assert "11.3%" in text
    assert "THR 8" in text
    assert "USER noel" in text
    assert "ST running" in text


def test_format_process_detail_handles_missing_selection():
    text = format_process_detail(None)

    assert "No process selected" in text
