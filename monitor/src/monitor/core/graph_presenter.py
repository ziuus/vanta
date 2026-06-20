"""Helpers for formatting graph/trend data in terminal sparklines."""


def auto_scale_range(values: list[float], padding: float = 0.05) -> tuple[float, float]:
    if not values:
        return 0.0, 100.0
    lo = min(values)
    hi = max(values)
    if abs(hi - lo) < 1e-9:
        lo -= 10.0
        hi += 10.0
    pad = (hi - lo) * padding
    return lo - pad, hi + pad


def format_graph_header(title: str, value: float, unit: str = "") -> str:
    if unit:
        line = f"{title:<12} {value:>6.1f}{unit}"
    else:
        line = f"{title:<12} {value:>6.1f}"
    return line


def _format_bytes(v: float) -> str:
    for suffix in ("", "KiB", "MiB", "GiB", "TiB"):
        if abs(v) < 1024.0:
            return f"{v:.1f} {suffix}"
        v /= 1024.0
    return f"{v:.1f} PiB"


def make_graph_label(values: list[float], bytestyle: bool = False) -> str:
    if not values:
        return "n/a"
    lo = min(values)
    hi = max(values)
    mid = (lo + hi) / 2.0
    if bytestyle:
        return f"{_format_bytes(lo)}  {_format_bytes(mid)}  {_format_bytes(hi)}"
    return f"{lo:.1f}  {mid:.1f}  {hi:.1f}"


def time_range_label(seconds: int) -> str:
    if seconds < 120:
        return f"{seconds // 60}m" if seconds >= 60 else "<1m"
    return f"{seconds // 60}m"
