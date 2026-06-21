"""Renderer helpers for config-driven dashboard widgets."""

from __future__ import annotations

import calendar as pycalendar
import math
import os
import random
import shutil
import subprocess
from dataclasses import dataclass
from datetime import datetime
from pathlib import Path
from time import monotonic
from typing import Any

BARS = "▁▂▃▄▅▆▇█"
MATRIX_CHARS = "01アイウエオカキクケコガギグゲゴザジズゼゾ\"+/*{}[]<>"
# Green gradient palette
MATRIX_DIM = "[#003300]"
MATRIX_MID = "[#008800]"
MATRIX_BRIGHT = "[#00ff41]"
MATRIX_HIGH = "[#33ff77]"
IMAGE_EXTENSIONS = {".png", ".jpg", ".jpeg", ".webp", ".gif", ".bmp"}
DEFAULT_WIDGET_REFRESH_INTERVALS: dict[str, float] = {
    "clock": 1.0,
    "calendar": 60.0,
    "matrix": 1.0,
    "music_viz": 1.0,
    "pstree": 5.0,
    "fastfetch": 60.0,
    "custom_text": 30.0,
    "image": 30.0,
    "wallpaper": 30.0,
    "yazi": 5.0,
}


@dataclass(slots=True)
class WidgetCacheEntry:
    content: str
    expires_at: float


class WidgetRenderCache:
    def __init__(self) -> None:
        self._entries: dict[str, WidgetCacheEntry] = {}

    def render(self, name: str, cfg: dict[str, Any], *, now: float | None = None) -> str:
        now = monotonic() if now is None else now
        cached = self._entries.get(name)
        if cached and now < cached.expires_at:
            return cached.content
        content = build_widget_content(name, cfg)
        ttl = widget_refresh_interval(name, cfg)
        self._entries[name] = WidgetCacheEntry(content=content, expires_at=now + ttl)
        return content

    def invalidate(self, name: str | None = None) -> None:
        if name is None:
            self._entries.clear()
        else:
            self._entries.pop(name, None)


def paginate_widgets(names: list[str], page_size: int = 3) -> list[list[str]]:
    if page_size <= 0:
        return [names[:]] if names else [[]]
    if not names:
        return [[]]
    return [names[i : i + page_size] for i in range(0, len(names), page_size)]


def widget_refresh_interval(name: str, cfg: dict[str, Any]) -> float:
    if name == "wallpaper":
        return max(1.0, float(cfg.get("interval", DEFAULT_WIDGET_REFRESH_INTERVALS.get(name, 5.0))))
    if "refresh_interval" in cfg:
        return max(1.0, float(cfg.get("refresh_interval", 5.0)))
    return max(1.0, float(DEFAULT_WIDGET_REFRESH_INTERVALS.get(name, 5.0)))


def build_clock_widget(cfg: dict[str, Any], now_text: str | None = None) -> str:
    dt = datetime.now() if now_text is None else datetime.fromisoformat(now_text)
    clock = dt.strftime("%H:%M:%S") if cfg.get("format", "24h") == "24h" else dt.strftime("%I:%M:%S %p")
    if cfg.get("show_date", True):
        return f"{clock}\n{dt.strftime('%Y-%m-%d')}"
    return clock


def build_calendar_widget(cfg: dict[str, Any], *, year: int | None = None, month: int | None = None) -> str:
    now = datetime.now()
    year = year or now.year
    month = month or now.month
    cal = pycalendar.TextCalendar(firstweekday=0)
    return cal.formatmonth(year, month).rstrip()


def build_custom_text_widget(cfg: dict[str, Any]) -> str:
    sections = cfg.get("sections", [])
    if not sections:
        return "No sections configured"
    blocks: list[str] = []
    for section in sections:
        title = section.get("title", "Untitled")
        content = section.get("content", "")
        blocks.append(f"[{title}]\n{content}")
    return "\n\n".join(blocks)


def build_matrix_widget(cfg: dict[str, Any], *, width: int = 22, height: int = 7, tick: int | None = None) -> str:
    """Animated matrix rain — characters shift with each tick, green gradient coloring."""
    density = max(0.1, float(cfg.get("density", 1.0)))
    tick = tick if tick is not None else int(datetime.now().timestamp())
    rng = random.Random(tick)
    lines: list[str] = []
    for row in range(height):
        line_chars: list[str] = []
        for col in range(width):
            if rng.random() < min(0.95, density * 0.4):
                c = rng.choice(MATRIX_CHARS)
                # Simulate a falling column: each row has different brightness
                if row == 0:
                    line_chars.append(f"{MATRIX_BRIGHT}{c}[/]")
                elif row < height // 3:
                    line_chars.append(f"{MATRIX_HIGH}{c}[/]")
                elif row < height * 2 // 3:
                    line_chars.append(f"{MATRIX_MID}{c}[/]")
                else:
                    line_chars.append(f"{MATRIX_DIM}{c}[/]")
            else:
                line_chars.append("  " if width > 30 else " ")
        lines.append("".join(line_chars))
    return "\n".join(lines)


def build_music_widget(cfg: dict[str, Any], *, tick: int | None = None, width: int = 20) -> str:
    bars = int(cfg.get("bars", 16))
    sensitivity = float(cfg.get("sensitivity", 0.5))
    tick = tick if tick is not None else int(datetime.now().timestamp())
    values = []
    for i in range(bars):
        val = (math.sin((tick / 2.0) + i * 0.55) + 1) / 2
        idx = min(len(BARS) - 1, max(0, int(val * sensitivity * (len(BARS) - 1) * 1.8)))
        values.append(BARS[idx])
    return "".join(values)


def build_pstree_widget(cfg: dict[str, Any]) -> str:
    max_depth = int(cfg.get("max_depth", 3))
    try:
        result = subprocess.run(
            ["pstree", "-p"],
            capture_output=True,
            text=True,
            timeout=1,
            check=False,
        )
    except FileNotFoundError:
        return "pstree not installed"
    lines = [line[:72] for line in result.stdout.splitlines()[:max_depth]]
    return "\n".join(lines) if lines else "pstree produced no output"


def build_fastfetch_widget(cfg: dict[str, Any]) -> str:
    try:
        result = subprocess.run(
            ["fastfetch", "--logo", "none", "--pipe", "false"],
            capture_output=True,
            text=True,
            timeout=1,
            check=False,
        )
    except FileNotFoundError:
        return "fastfetch not installed"
    lines = [line.strip() for line in result.stdout.splitlines() if line.strip()][:6]
    return "\n".join(lines) if lines else "fastfetch produced no output"


def build_yazi_widget(cfg: dict[str, Any]) -> str:
    cwd = Path(os.path.expanduser(str(cfg.get("cwd", "~"))))
    if not cwd.exists():
        return f"{cwd}\nmissing directory"
    entries = sorted(cwd.iterdir(), key=lambda p: (not p.is_dir(), p.name.lower()))[:10]
    lines = [str(cwd)]
    for entry in entries:
        suffix = "/" if entry.is_dir() else ""
        lines.append(f"{entry.name}{suffix}")
    return "\n".join(lines)


def build_image_widget(cfg: dict[str, Any]) -> str:
    path = Path(os.path.expanduser(str(cfg.get("path", ""))))
    if not str(path) or str(path) == ".":
        return "No image configured"
    if not path.exists():
        return f"Image missing\n{path}"
    # Try chafa for ANSI image rendering (works in any terminal)
    chafa_bin = shutil.which("chafa")
    if chafa_bin:
        try:
            result = subprocess.run(
                [chafa_bin, "--symbols", "all", "--size", "24x12", str(path)],
                capture_output=True, text=True, timeout=5,
            )
            if result.returncode == 0 and result.stdout.strip():
                # Strip trailing newlines, chafa tends to add an extra blank line
                return result.stdout.rstrip("\n")
        except (subprocess.TimeoutExpired, OSError):
            pass
    return f"Image\n{path.name}\n{path.stat().st_size // 1024} KiB"


def build_wallpaper_widget(cfg: dict[str, Any]) -> str:
    directory = Path(os.path.expanduser(str(cfg.get("directory", "~/Pictures"))))
    if not directory.exists():
        return f"Wallpaper dir not found\n{directory}"
    images = sorted([p for p in directory.iterdir() if p.suffix.lower() in IMAGE_EXTENSIONS])
    if not images:
        return f"Wallpaper dir empty\n{directory}"
    interval = max(1, int(cfg.get("interval", 300)))
    index = int(datetime.now().timestamp() // interval) % len(images)
    current = images[index]
    return f"Wallpaper\n{current.name}\n{index + 1}/{len(images)} in {directory.name}"


def build_widget_content(name: str, cfg: dict[str, Any]) -> str:
    if name == "clock":
        return build_clock_widget(cfg)
    if name == "calendar":
        return build_calendar_widget(cfg)
    if name == "matrix":
        return build_matrix_widget(cfg)
    if name == "music_viz":
        return build_music_widget(cfg)
    if name == "pstree":
        return build_pstree_widget(cfg)
    if name == "fastfetch":
        return build_fastfetch_widget(cfg)
    if name == "custom_text":
        return build_custom_text_widget(cfg)
    if name == "image":
        return build_image_widget(cfg)
    if name == "wallpaper":
        return build_wallpaper_widget(cfg)
    if name == "yazi":
        return build_yazi_widget(cfg)
    return f"Unsupported widget\n{name}"
