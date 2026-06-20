"""Typed config loader for the modular Vanta dashboard."""

from __future__ import annotations

import json
from dataclasses import dataclass
from pathlib import Path
from typing import Any


DEFAULT_CONFIG: dict[str, Any] = {
    "ui": {"refresh_rate": 1.0, "theme": "light"},
    "process": {"show_kernel": False, "max_display": 15, "auto_refresh": True},
    "audio": {"simulated": True, "sensitivity": 0.5},
    "widgets": {
        "dashboard": {"enabled": True, "columns": ["cpu", "gpu", "mem", "disk", "net"]},
        "clock": {"enabled": True, "format": "24h", "show_date": True},
        "calendar": {"enabled": True},
        "matrix": {"enabled": True, "speed": 1.0, "density": 1.0},
        "music_viz": {"enabled": True, "bars": 32, "sensitivity": 0.5},
        "pstree": {"enabled": True, "max_depth": 3},
        "fastfetch": {"enabled": True, "refresh_interval": 60},
        "custom_text": {
            "enabled": True,
            "sections": [
                {"title": "TODO", "content": "• Refactor widgets\n• Add themes\n• Write tests"},
                {"title": "Links", "content": "• github.com/ziuus/vanta"},
            ],
        },
        "image": {"enabled": False, "path": ""},
        "wallpaper": {"enabled": False, "interval": 300, "directory": "~/Pictures"},
        "yazi": {"enabled": True, "cwd": "~"},
        "process_manager": {"enabled": True, "show_kernel": False, "max_display": 15},
        "system_stats": {"enabled": True},
    },
}

HIDDEN_WIDGETS = {"dashboard", "system_stats", "process_manager"}


@dataclass(slots=True)
class UIConfig:
    refresh_rate: float = 1.0
    theme: str = "light"


@dataclass(slots=True)
class ProcessConfig:
    show_kernel: bool = False
    max_display: int = 15
    auto_refresh: bool = True


@dataclass(slots=True)
class WidgetConfig:
    name: str
    enabled: bool
    settings: dict[str, Any]

    def get(self, key: str, default: Any = None) -> Any:
        return self.settings.get(key, default)


@dataclass(slots=True)
class DashboardConfig:
    raw: dict[str, Any]
    ui: UIConfig
    process: ProcessConfig
    audio: dict[str, Any]
    widgets: dict[str, WidgetConfig]

    def widget(self, name: str) -> WidgetConfig:
        return self.widgets.get(name, WidgetConfig(name=name, enabled=False, settings={}))

    def enabled_widget_names(self) -> list[str]:
        return [name for name, cfg in self.widgets.items() if cfg.enabled]

    def enabled_extra_widget_names(self) -> list[str]:
        return [name for name in self.enabled_widget_names() if name not in HIDDEN_WIDGETS]

    def page_size_for_width(self, width: int) -> int:
        if width >= 160:
            return 4
        if width >= 110:
            return 3
        if width >= 76:
            return 2
        return 1

    def show_history_for_height(self, height: int) -> bool:
        return height >= 34

    def compact_mode_for_height(self, height: int) -> bool:
        return height < 30

    def ultra_compact_mode_for_height(self, height: int) -> bool:
        return height < 20


def _deep_merge(base: dict[str, Any], override: dict[str, Any]) -> dict[str, Any]:
    merged = dict(base)
    for key, value in override.items():
        if isinstance(value, dict) and isinstance(merged.get(key), dict):
            merged[key] = _deep_merge(merged[key], value)
        else:
            merged[key] = value
    return merged


def _build_dashboard_config(raw: dict[str, Any]) -> DashboardConfig:
    ui_raw = raw.get("ui", {})
    process_raw = raw.get("process", {})
    widgets_raw = raw.get("widgets", {})
    widgets: dict[str, WidgetConfig] = {}
    for name, settings in widgets_raw.items():
        widget_settings = dict(settings)
        widgets[name] = WidgetConfig(
            name=name,
            enabled=bool(widget_settings.get("enabled", False)),
            settings=widget_settings,
        )
    return DashboardConfig(
        raw=raw,
        ui=UIConfig(
            refresh_rate=float(ui_raw.get("refresh_rate", 1.0)),
            theme=str(ui_raw.get("theme", "light")),
        ),
        process=ProcessConfig(
            show_kernel=bool(process_raw.get("show_kernel", False)),
            max_display=int(process_raw.get("max_display", 15)),
            auto_refresh=bool(process_raw.get("auto_refresh", True)),
        ),
        audio=dict(raw.get("audio", {})),
        widgets=widgets,
    )


def load_dashboard_config(path: Path) -> DashboardConfig:
    if path.exists():
        with path.open() as f:
            loaded = json.load(f)
    else:
        loaded = {}
    merged = _deep_merge(DEFAULT_CONFIG, loaded)
    return _build_dashboard_config(merged)
