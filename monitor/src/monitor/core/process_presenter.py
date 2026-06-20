from monitor.core.models import ProcessRow


def _pct_color(value: float) -> str:
    if value < 50:
        return "green"
    if value < 80:
        return "yellow"
    return "red"


def format_process_status(
    *,
    sort_col: str,
    descending: bool,
    query: str,
    total_rows: int,
    selected_pid: int | None,
) -> str:
    direction = "desc" if descending else "asc"
    filter_text = query.strip() or "none"
    pid_text = str(selected_pid) if selected_pid is not None else "none"
    return (
        f"sort: {sort_col} {direction}  |  "
        f"filter: {filter_text}  |  "
        f"rows: {total_rows}  |  "
        f"pid: {pid_text}"
    )


def format_process_detail(row: ProcessRow | None) -> str:
    if row is None:
        return "No process selected"
    cpu_color = _pct_color(row.cpu_percent)
    mem_color = _pct_color(row.memory_percent)
    return (
        f"{row.name}  |  PID {row.pid}  |  "
        f"CPU [{cpu_color}]{row.cpu_percent:.1f}%[/]  |  "
        f"MEM [{mem_color}]{row.memory_percent:.1f}%[/]  |  "
        f"THR {row.threads}  |  "
        f"USER {row.username or 'n/a'}  |  "
        f"ST {row.status}"
    )
