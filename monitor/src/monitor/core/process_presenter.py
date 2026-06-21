"""Formatting helpers for process lists and detail views.

All colour-producing functions accept an optional *pal* dict so they
are theme-aware.
"""

from monitor.core.models import ProcessRow
from monitor.core.theme import LIGHT

_DEFAULT = LIGHT


def _pct_color(value: float, pal: dict[str, str] | None = None) -> str:
    p = pal or _DEFAULT
    if value < 50:
        return p.get("green", "green")
    if value < 80:
        return p.get("yellow", "yellow")
    return p.get("red", "red")


def format_process_status(
    *,
    sort_col: str,
    descending: bool,
    query: str,
    total_rows: int,
    visible_rows: int = 0,
    selected_pid: int | None = None,
    pal: dict[str, str] | None = None,
) -> str:
    # Plain text status bar without Textual markup, matching test expectations.
    direction = "desc" if descending else "asc"
    filter_text = query.strip() or "none"
    pid_text = str(selected_pid) if selected_pid is not None else "none"
    # Build components
    parts = []
    parts.append(f"sort: {sort_col} {direction}")
    parts.append(f"filter: {filter_text}")
    if visible_rows:
        parts.append(f"rows: {visible_rows}/{total_rows}")
    else:
        parts.append(f"rows: {total_rows}")
    parts.append(f"pid: {pid_text}")
    return "  ".join(parts)


def format_process_detail(
    row: ProcessRow | None,
    detail: dict | None = None,
    pal: dict[str, str] | None = None,
) -> str:
    if row is None:
        return "No process selected"
    # Build a concise, plain-text summary
    parts = []
    parts.append(row.name)
    parts.append(f"PID {row.pid}")
    parts.append(f"CPU {row.cpu_percent:.1f}%")
    parts.append(f"MEM {row.memory_percent:.1f}%")
    parts.append(f"THR {row.threads}")
    parts.append(f"USER {row.username or 'n/a'}")
    parts.append(f"ST {row.status}")
    base = "  ".join(parts)
    if not detail:
        return base
    # Include extended details in a simple key: value format
    extra_parts = []
    for key in ["exe", "children", "fds", "connections", "nice", "memory_rss"]:
        if key in detail and detail[key]:
            val = detail[key]
            if key == "memory_rss":
                # Convert bytes to MiB
                val = f"{val / (1024**2):.0f} MiB"
            extra_parts.append(f"{key}: {val}")
    if extra_parts:
        return base + "\n" + ", ".join(extra_parts)
    return base
