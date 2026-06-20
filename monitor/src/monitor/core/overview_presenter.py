"""Presenters for the dashboard overview panels."""

from monitor.core.models import ProcessRow, SystemSnapshot


def _bar_color(percent: float) -> str:
    """Rich markup color name based on utilization threshold."""
    if percent < 50:
        return "green"
    if percent < 80:
        return "yellow"
    return "red"


def compact_bar(percent: float, width: int = 16) -> str:
    """Color-coded horizontal bar with Rich markup."""
    percent = max(0.0, min(100.0, percent))
    filled = round((percent / 100.0) * width)
    color = _bar_color(percent)
    bar = "█" * filled + "·" * (width - filled)
    return f"[{color}]{bar}[/]"


def format_bytes_binary(value: int) -> str:
    units = ["B", "KiB", "MiB", "GiB", "TiB"]
    current = float(value)
    for unit in units:
        if current < 1024 or unit == units[-1]:
            if unit == "B":
                return f"{int(current)} B"
            return f"{current:.1f} {unit}"
        current /= 1024.0
    return f"{current:.1f} TiB"


def format_rate_binary(value: float) -> str:
    units = ["B/s", "KiB/s", "MiB/s", "GiB/s"]
    current = float(value)
    for unit in units:
        if current < 1024 or unit == units[-1]:
            return f"{current:.1f} {unit}"
        current /= 1024.0
    return f"{current:.1f} GiB/s"


def _cpu_panel(snapshot: SystemSnapshot) -> str:
    freq = (
        f"{snapshot.cpu.frequency_mhz / 1000:.2f} GHz"
        if snapshot.cpu.frequency_mhz
        else "n/a"
    )
    lines = [
        f"CPU  {snapshot.cpu.total_percent:.1f}%  load {snapshot.cpu.load_avg_1m:.2f}  "
        f"Freq {freq}  Cores {snapshot.cpu.core_count}",
        compact_bar(snapshot.cpu.total_percent, width=28),
    ]
    # Per-core bars in a 2-column grid
    cores = snapshot.cpu.per_core_percent
    pairs = []
    for i in range(0, len(cores), 2):
        left = cores[i]
        right = cores[i + 1] if i + 1 < len(cores) else None
        left_bar = f"c{i} {left:>5.1f}% {compact_bar(left, width=10)}"
        if right is not None:
            right_bar = f"c{i+1} {right:>5.1f}% {compact_bar(right, width=10)}"
            pairs.append(f"{left_bar:<30}{right_bar}")
        else:
            pairs.append(left_bar)
    lines.extend(pairs)
    return "\n".join(lines)


def _memory_panel(snapshot: SystemSnapshot) -> str:
    mem_color = _bar_color(snapshot.memory.percent)
    return "\n".join(
        [
            f"MEM  {snapshot.memory.percent:.1f}%  "
            f"Used {format_bytes_binary(snapshot.memory.used_bytes)} / "
            f"{format_bytes_binary(snapshot.memory.total_bytes)}",
            compact_bar(snapshot.memory.percent, width=24),
            f"Avail {format_bytes_binary(snapshot.memory.available_bytes)}  "
            f"Swap [{_bar_color(snapshot.memory.swap_percent)}]{snapshot.memory.swap_percent:.1f}%[/]",
        ]
    )


def _network_panel(snapshot: SystemSnapshot) -> str:
    return "\n".join(
        [
            f"Down  {format_rate_binary(snapshot.network.download_bps)}",
            f"Up    {format_rate_binary(snapshot.network.upload_bps)}",
            f"Recv  {format_bytes_binary(snapshot.network.bytes_recv)}",
            f"Sent  {format_bytes_binary(snapshot.network.bytes_sent)}",
        ]
    )


def _system_panel(snapshot: SystemSnapshot) -> str:
    gpu_lines: list[str] = []
    if snapshot.gpu:
        gpu_color = _bar_color(snapshot.gpu.util_percent)
        mem_color = _bar_color(snapshot.gpu.memory_percent)
        vram_used = format_bytes_binary(snapshot.gpu.memory_used_bytes)
        vram_total = format_bytes_binary(snapshot.gpu.memory_total_bytes)
        gpu_lines = [
            f"GPU  {snapshot.gpu.util_percent:.0f}%  "
            f"{snapshot.gpu.temperature_c:.0f}C",
            compact_bar(snapshot.gpu.util_percent, width=20),
            f"VRAM {snapshot.gpu.memory_percent:.0f}%  "
            f"{vram_used} / {vram_total}",
            compact_bar(snapshot.gpu.memory_percent, width=20),
        ]
    temp = (
        "n/a"
        if snapshot.temperature_c is None
        else f"{snapshot.temperature_c:.0f}C"
    )
    uptime_h = snapshot.uptime_seconds / 3600.0
    return "\n".join(
        [
            f"Proc {snapshot.process_count}  Threads {snapshot.thread_count}",
            f"Temp {temp}  Up {uptime_h:.1f}h",
            *gpu_lines,
        ]
    )


def _disk_panel(snapshot: SystemSnapshot) -> str:
    lines = []
    for idx, disk in enumerate(snapshot.disks[:4]):
        color = _bar_color(disk.percent)
        lines.append(
            f"{disk.mountpoint:<8}{disk.percent:>5.1f}%  "
            f"{compact_bar(disk.percent, width=10)}  "
            f"free {format_bytes_binary(disk.free_bytes)}"
        )
    # Show aggregate disk IO on the first disk's IO field
    if snapshot.disks:
        io = snapshot.disks[0]
        if io.io_read_bps > 0 or io.io_write_bps > 0:
            lines.append(
                f"Read {format_rate_binary(io.io_read_bps)}  "
                f"Write {format_rate_binary(io.io_write_bps)}"
            )
    return "\n".join(lines)


def make_process_preview(rows: list[ProcessRow], limit: int = 8) -> str:
    header = "PID     NAME                 CPU%   MEM%   ST"
    body = []
    for row in rows[:limit]:
        cpu_color = _bar_color(row.cpu_percent)
        mem_color = _bar_color(row.memory_percent)
        body.append(
            f"{row.pid:<7} {row.name[:20]:<20} "
            f"[{cpu_color}]{row.cpu_percent:>5.1f}[/]  "
            f"[{mem_color}]{row.memory_percent:>5.1f}[/]  "
            f"{row.status[:2]}"
        )
    return "\n".join([header, *body])


def make_overview_panels(snapshot: SystemSnapshot) -> dict[str, str]:
    return {
        "cpu": _cpu_panel(snapshot),
        "memory": _memory_panel(snapshot),
        "network": _network_panel(snapshot),
        "system": _system_panel(snapshot),
        "disks": _disk_panel(snapshot),
    }
