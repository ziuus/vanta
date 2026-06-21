"""Presenters for the dashboard overview panels.
All colour-producing functions accept an optional *pal* dict so the
caller can inject the active theme.  When omitted a light default is
used, keeping existing callers (and tests) working without changes.
"""

from __future__ import annotations

from monitor.core.models import ProcessRow, SystemSnapshot
from monitor.core.theme import LIGHT

# ---------------------------------------------------------------------------
# default palette — used when callers don't explicitly pass one
# ---------------------------------------------------------------------------
_DEFAULT = LIGHT


def _bar_color(percent: float, pal: dict[str, str] | None = None) -> str:
    """Return a hex colour (or named colour) for the given utilisation."""
    p = pal or _DEFAULT
    if percent < 50:
        return p.get("green", "green")
    if percent < 80:
        return p.get("yellow", "yellow")
    return p.get("red", "red")


def compact_bar(
    percent: float, width: int = 16, pal: dict[str, str] | None = None
) -> str:
    """Colour-coded horizontal bar with Rich markup (hex colour)."""
    percent = max(0.0, min(100.0, percent))
    filled = round((percent / 100.0) * width)
    color = _bar_color(percent, pal)
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


# ---------------------------------------------------------------------------
# Panel formatters — all accept an optional *pal*
# ---------------------------------------------------------------------------


def _cpu_panel(snapshot: SystemSnapshot, pal: dict[str, str] | None = None) -> str:
    p = pal or _DEFAULT
    freq = (
        f"{snapshot.cpu.frequency_mhz / 1000:.2f} GHz"
        if snapshot.cpu.frequency_mhz
        else "n/a"
    )
    pwr = f"  Pkg {snapshot.cpu_power_watts:.1f}W" if snapshot.cpu_power_watts > 0 else ""
    dot_c = _bar_color(snapshot.cpu.total_percent, p)
    lines = [
        f"[{p['accent']}]🖥 CPU[/]  [{dot_c}]●[/]  "
        f"[{p['text_muted']}]{snapshot.cpu.total_percent:.1f}%  "
        f"load {snapshot.cpu.load_avg_1m:.2f}  "
        f"Freq {freq}  Cores {snapshot.cpu.core_count}{pwr}[/]",
        compact_bar(snapshot.cpu.total_percent, width=32, pal=p),
    ]
    # Per-core bars as a 2-column grid
    cores = snapshot.cpu.per_core_percent
    pairs = []
    for i in range(0, len(cores), 2):
        left = cores[i]
        right = cores[i + 1] if i + 1 < len(cores) else None
        left_bar = (
            f"[{p['text_dim']}]c{i}[/] {left:>5.1f}% "
            f"{compact_bar(left, width=10, pal=p)}"
        )
        if right is not None:
            right_bar = (
                f"[{p['text_dim']}]c{i+1}[/] {right:>5.1f}% "
                f"{compact_bar(right, width=10, pal=p)}"
            )
            pairs.append(f"{left_bar:<38}{right_bar}")
        else:
            pairs.append(left_bar)
    lines.extend(pairs)
    return "\n".join(lines)


def _memory_panel(snapshot: SystemSnapshot, pal: dict[str, str] | None = None) -> str:
    p = pal or _DEFAULT
    dot_c = _bar_color(snapshot.memory.percent, p)
    return "\n".join(
        [
            f"[{p['accent']}]🧠 Memory[/]  [{dot_c}]●[/]  "
            f"[{p['text_muted']}]{snapshot.memory.percent:.1f}%  "
            f"Used {format_bytes_binary(snapshot.memory.used_bytes)} / "
            f"{format_bytes_binary(snapshot.memory.total_bytes)}[/]",
            compact_bar(snapshot.memory.percent, width=30, pal=p),
            f"[{p['text']}]Avail {format_bytes_binary(snapshot.memory.available_bytes)}  "
            f"Swap [{_bar_color(snapshot.memory.swap_percent, p)}]"
            f"{snapshot.memory.swap_percent:.1f}%[/]",
            compact_bar(snapshot.memory.swap_percent, width=30, pal=p),
        ]
    )


def _network_panel(snapshot: SystemSnapshot, pal: dict[str, str] | None = None) -> str:
    p = pal or _DEFAULT
    lines = [
        f"[{p['accent']}]🌐 Network[/]  [{p['green']}]●[/]  "
        f"[{p['text_muted']}]Down {format_rate_binary(snapshot.network.download_bps)}  "
        f"Up {format_rate_binary(snapshot.network.upload_bps)}[/]",
        f"[{p['text_muted']}]Recv  {format_bytes_binary(snapshot.network.bytes_recv)}  "
        f"Sent  {format_bytes_binary(snapshot.network.bytes_sent)}[/]",
    ]
    # Show top 2 interfaces
    ifaces = sorted(
        snapshot.network.interfaces,
        key=lambda i: i.download_bps + i.upload_bps,
        reverse=True,
    )
    for iface in ifaces[:2]:
        if iface.download_bps > 0 or iface.upload_bps > 0:
            up = format_rate_binary(iface.upload_bps)
            down = format_rate_binary(iface.download_bps)
            lines.append(
                f"[{p['text_dim']}]  {iface.name[:6]:<6}[/]"
                f"[{p['text']}]↑{up:<10}↓{down}[/]"
            )
    return "\n".join(lines)


def _system_panel(snapshot: SystemSnapshot, pal: dict[str, str] | None = None) -> str:
    p = pal or _DEFAULT

    # Overall health dot
    has_issue = snapshot.temperature_c is not None and snapshot.temperature_c > 80
    health_dot = f"[{p['red'] if has_issue else p['green']}]●[/]"
    gpu_lines: list[str] = []
    if snapshot.gpu:
        g = snapshot.gpu
        vram_used = format_bytes_binary(g.memory_used_bytes)
        vram_total = format_bytes_binary(g.memory_total_bytes)
        tag = g.name.split()[-1] if g.name else "GPU"
        gpu_lines.append(f"[{p['accent']}]● {tag}[/]  "
                         f"[{p['text']}]{g.util_percent:.0f}%  "
                         f"{g.temperature_c:.0f}°C[/]")
        gpu_lines.append(compact_bar(g.util_percent, width=16, pal=p))
        if g.power_watts > 0:
            pwr = f"{g.power_watts:.1f}W"
            if g.power_max_watts > 0:
                pwr += f"/{g.power_max_watts:.0f}W"
            gpu_lines.append(f"[{p['text']}]Power {pwr}  "
                             f"Clk {g.clock_graphics_mhz}/{g.clock_mem_mhz}MHz[/]")
        gpu_lines.append(
            f"[{p['text_muted']}]VRAM  {g.memory_percent:.0f}%  "
            f"{vram_used}/{vram_total}[/]"
        )
        gpu_lines.append(compact_bar(g.memory_percent, width=16, pal=p))
        if g.encoder_util_percent > 0 or g.decoder_util_percent > 0:
            gpu_lines.append(
                f"[{p['text_dim']}]Enc {g.encoder_util_percent:.0f}%  "
                f"Dec {g.decoder_util_percent:.0f}%[/]"
            )

    # Battery
    bat_lines: list[str] = []
    if snapshot.battery:
        bat = snapshot.battery
        icon = "🔌" if bat.status in ("Charging", "Full") else "🔋"
        time_str = ""
        if bat.time_to_empty_min and bat.time_to_empty_min > 0:
            time_str = f"  {bat.time_to_empty_min:.0f}m"
        elif bat.time_to_full_min and bat.time_to_full_min > 0:
            time_str = f"  {bat.time_to_full_min:.0f}m"
        bat_lines.append(
            f"[{p['text']}]{icon} {bat.percent:.0f}%"
            f"{' ' + bat.status[:6] if bat.status != 'Unknown' else ''}"
            f"{time_str}"
            f"{f' {bat.power_watts:.1f}W' if bat.power_watts > 0 else ''}[/]"
        )

    temp = (
        "n/a" if snapshot.temperature_c is None
        else f"{snapshot.temperature_c:.0f}°C"
    )
    uptime_h = snapshot.uptime_seconds / 3600.0
    sys_line = (
        f"[{p['accent']}]⚙ System[/]  {health_dot}  "
        f"[{p['text_muted']}]{snapshot.process_count} procs, "
        f"{snapshot.thread_count} threads[/]  "
        f"[{p['text_dim']}]{temp}  up {uptime_h:.1f}h[/]"
    )

    parts = [sys_line]
    if gpu_lines:
        parts.append("")
        parts.extend(gpu_lines)
    if bat_lines:
        parts.append("")
        parts.extend(bat_lines)
    return "\n".join(parts)


def _disk_panel(snapshot: SystemSnapshot, pal: dict[str, str] | None = None) -> str:
    p = pal or _DEFAULT
    # Status dot based on max disk usage
    max_disk = max((d.percent for d in snapshot.disks), default=0)
    dot_c = _bar_color(max_disk, p)
    lines = [f"[{p['accent']}]💾 Disks[/]  [{dot_c}]●[/]  [{p['text_muted']}]{len(snapshot.disks)} mounts[/]"]
    for idx, disk in enumerate(snapshot.disks[:4]):
        dev = f"{disk.device[:4]}:" if disk.device else ""
        c = _bar_color(disk.percent, p)
        line = (
            f"[{p['text_dim']}]{dev:<4}[/]"
            f"[{p['text']}]{disk.mountpoint:<8}"
            f"[{c}]{disk.percent:>5.1f}%[/]"
            f"  {compact_bar(disk.percent, width=4, pal=p)}  "
            f"[{p['text_muted']}]{format_bytes_binary(disk.free_bytes)}[/]"
        )
        if idx == 0 and disk.io_busy_percent > 0:
            busy = _bar_color(disk.io_busy_percent, p)
            line += f"  [{busy}]{disk.io_busy_percent:.0f}% bsy[/]"
        lines.append(line)
    # Per-device disk IO
    active = sorted(
        snapshot.disks,
        key=lambda d: d.io_read_bps + d.io_write_bps,
        reverse=True,
    )
    top = active[0] if active else None
    if top and (top.io_read_bps > 0 or top.io_write_bps > 0):
        tag = f"{top.device}:" if top.device else ""
        lines.append(
            f"[{p['text_dim']}]{tag}R {format_rate_binary(top.io_read_bps)}  "
            f"W {format_rate_binary(top.io_write_bps)}[/]"
        )
    return "\n".join(lines)


def make_process_preview(
    rows: list[ProcessRow], limit: int = 8, pal: dict[str, str] | None = None
) -> str:
    p = pal or _DEFAULT
    header = f"[{p['text_muted']}]{'PID':>7}  NAME{'':<17} CPU%   MEM%   ST  THR[/]"
    body = []
    for row in rows[:limit]:
        cpu_c = _bar_color(row.cpu_percent, p)
        mem_c = _bar_color(row.memory_percent, p)
        body.append(
            f"[{p['text_dim']}]  {row.pid:<6}[/]"
            f"[{p['text']}] {row.name[:20]:<20} [/]"
            f"[{cpu_c}]{row.cpu_percent:>5.1f}[/]  "
            f"[{mem_c}]{row.memory_percent:>5.1f}[/]  "
            f"[{p['text_dim']}]{row.status[:2]:<2}  {row.threads:>2}[/]"
        )
    return "\n".join([header, *body])


# ---------------------------------------------------------------------------
# nowplaying / gpu fallback
# ---------------------------------------------------------------------------
BARS = "▁▂▃▄▅▆▇█"


def _nowplaying_text(snapshot: SystemSnapshot, pal: dict[str, str]) -> str:
    """Compact GPU / battery / nowplaying info line."""
    from datetime import datetime
    import math

    p = pal

    # ----- media -----
    from monitor.core.media import MediaDetector

    media = MediaDetector()
    np_info = media.detect()
    if np_info and np_info["status"] != "Stopped":
        icon = "▶" if np_info["status"] == "Playing" else "⏸"
        t = datetime.now().timestamp()
        bar_count = 8
        bar_chars = [
            BARS[
                min(
                    len(BARS) - 1,
                    int(((math.sin(t + i * 0.55) + 1) / 2) * (len(BARS) - 1)),
                )
            ]
            for i in range(bar_count)
        ]
        return (
            f"[{p['accent']}]{icon}[/] "
            f"[{p['text']}]{np_info['title'][:20]}[/] "
            f"[{p['text_muted']}]{np_info['artist'][:12]}[/]"
            f"{' · ' + np_info['album'][:12] if np_info.get('album') else ''}  "
            f"[{p['green']}]{''.join(bar_chars)}[/]  "
            f"[{p['text_dim']}][z]⏯ [x]⏭ [c]⏮[/]"
        )

    # ----- GPU fallback -----
    parts: list[str] = []
    if snapshot.gpu:
        g = snapshot.gpu
        tag = g.name.split()[-1] if g.name else "GPU"
        parts.append(
            f"[{p['accent']}]● {tag}[/] "
            f"[{p['text']}]{g.util_percent:.0f}%  "
            f"{g.temperature_c:.0f}°C  "
            f"P {g.power_watts:.1f}W[/]"
        )
        if g.clock_graphics_mhz > 0:
            parts.append(
                f"[{p['text_muted']}]Clk {g.clock_graphics_mhz}/{g.clock_mem_mhz}MHz  "
                f"M {g.memory_util_percent:.0f}%[/]"
            )
    # Battery
    if snapshot.battery:
        bat = snapshot.battery
        icon = "🔌" if bat.status in ("Charging", "Full") else "🔋"
        parts.append(
            f"[{p['text']}]{icon} {bat.percent:.0f}%[/]"
        )
    if not parts:
        parts.append(f"[{p['text_muted']}]● idle[/]")
    return "  │  ".join(parts)


# ---------------------------------------------------------------------------
# public API
# ---------------------------------------------------------------------------


def make_overview_panels(
    snapshot: SystemSnapshot, pal: dict[str, str] | None = None
) -> dict[str, str]:
    return {
        "cpu": _cpu_panel(snapshot, pal),
        "memory": _memory_panel(snapshot, pal),
        "network": _network_panel(snapshot, pal),
        "system": _system_panel(snapshot, pal),
        "disks": _disk_panel(snapshot, pal),
    }
