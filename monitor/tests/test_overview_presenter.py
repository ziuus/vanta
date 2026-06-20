from monitor.core.models import (
    CpuSnapshot,
    DiskSnapshot,
    GpuSnapshot,
    MemorySnapshot,
    NetworkSnapshot,
    ProcessRow,
    SystemSnapshot,
)
from monitor.core.overview_presenter import (
    compact_bar,
    format_bytes_binary,
    format_rate_binary,
    make_overview_panels,
    make_process_preview,
)


def sample_snapshot() -> SystemSnapshot:
    return SystemSnapshot(
        cpu=CpuSnapshot(
            total_percent=67.4,
            per_core_percent=[10.0, 25.0, 50.0, 75.0, 99.0, 5.0, 33.0, 88.0],
            load_avg_1m=1.82,
            frequency_mhz=2894.0,
            core_count=8,
        ),
        memory=MemorySnapshot(
            percent=73.5,
            used_bytes=12 * 1024**3,
            total_bytes=16 * 1024**3,
            available_bytes=4 * 1024**3,
            swap_percent=18.0,
        ),
        disks=[
            DiskSnapshot(
                mountpoint="/",
                percent=61.0,
                used_bytes=610 * 1024**3,
                free_bytes=390 * 1024**3,
                total_bytes=1000 * 1024**3,
            ),
            DiskSnapshot(
                mountpoint="/home",
                percent=44.0,
                used_bytes=440 * 1024**3,
                free_bytes=560 * 1024**3,
                total_bytes=1000 * 1024**3,
            ),
        ],
        network=NetworkSnapshot(
            upload_bps=3 * 1024**2,
            download_bps=18 * 1024**2,
            bytes_sent=120 * 1024**3,
            bytes_recv=900 * 1024**3,
        ),
        gpu=GpuSnapshot(
            util_percent=54.0,
            temperature_c=71.0,
            memory_percent=62.0,
            memory_used_bytes=4960 * 1024**2,
            memory_total_bytes=8192 * 1024**2,
        ),
        process_count=318,
        thread_count=1490,
        uptime_seconds=36_540,
        temperature_c=57.0,
    )


def test_compact_bar_renders_requested_width():
    bar = compact_bar(50.0, width=10)
    # compact_bar now returns Rich markup, so check the plain-text bar chars
    import re
    plain = re.sub(r"\[/?\w*\]", "", bar)
    assert len(plain) == 10
    assert plain.count("█") == 5


def test_format_bytes_binary_uses_gib():
    assert format_bytes_binary(3 * 1024**3) == "3.0 GiB"


def test_format_rate_binary_uses_mib_per_second():
    assert format_rate_binary(18 * 1024**2) == "18.0 MiB/s"


def test_make_overview_panels_returns_dense_sections():
    panels = make_overview_panels(sample_snapshot())

    assert set(panels) == {"cpu", "memory", "network", "system", "disks"}
    assert "CPU  67.4%" in panels["cpu"]
    assert "c0" in panels["cpu"]
    assert "c7" in panels["cpu"]
    assert "MEM  73.5%" in panels["memory"]
    assert "Swap" in panels["memory"]
    assert "18.0 MiB/s" in panels["network"]
    assert "3.0 MiB/s" in panels["network"]
    assert "GPU" in panels["system"]
    assert "Proc 318" in panels["system"]
    assert "/" in panels["disks"] and "61.0%" in panels["disks"]
    assert "/home" in panels["disks"] and "44.0%" in panels["disks"]


def test_make_overview_panels_handles_missing_gpu():
    snap = sample_snapshot()
    snap.gpu = None

    panels = make_overview_panels(snap)

    assert "Temp 57C" in panels["system"]


def test_make_process_preview_returns_dense_ranked_lines():
    rows = [
        ProcessRow(4242, "python", 72.4, 11.3, "running", 8, "noel"),
        ProcessRow(881, "firefox", 36.5, 8.2, "sleeping", 24, "noel"),
        ProcessRow(1, "systemd", 0.0, 0.1, "sleeping", 1, "root"),
    ]

    text = make_process_preview(rows, limit=2)

    assert "PID" in text
    assert "python" in text
    assert "firefox" in text
    assert "systemd" not in text
    assert "72.4" in text
    assert "11.3" in text