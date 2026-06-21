#!/usr/bin/env python3
"""Vanta Monitor web dashboard and JSON API."""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any

from flask import Flask, jsonify, request, send_file
from flask_cors import CORS

from monitor.core.collectors import SystemCollector
from monitor.core.dashboard_config import load_dashboard_config
from monitor.core.process_service import ProcessService

try:
    import pynvml

    PYNVML_AVAILABLE = True
except ImportError:
    PYNVML_AVAILABLE = False


DEFAULT_PORT = 5001
CONFIG_PATH = Path(__file__).parent.parent.parent / "config.json"
HTML_DIR = Path(__file__).parent

_gpu_handle = None
if PYNVML_AVAILABLE:
    try:
        pynvml.nvmlInit()
        _gpu_handle = pynvml.nvmlDeviceGetHandleByIndex(0)
    except Exception:
        PYNVML_AVAILABLE = False
        _gpu_handle = None


app = Flask(__name__)
CORS(app)
collector = SystemCollector()
process_service = ProcessService()


def load_config() -> dict[str, Any]:
    if CONFIG_PATH.exists():
        with CONFIG_PATH.open() as f:
            return json.load(f)
    return load_dashboard_config(CONFIG_PATH).raw


config = load_config()


def _bool_arg(value: Any, *, default: bool = False) -> bool:
    if value is None:
        return default
    if isinstance(value, bool):
        return value
    return str(value).strip().lower() in {"1", "true", "yes", "on"}


def _int_arg(value: Any, *, default: int, minimum: int | None = None, maximum: int | None = None) -> int:
    try:
        result = int(value)
    except (TypeError, ValueError):
        result = default
    if minimum is not None:
        result = max(minimum, result)
    if maximum is not None:
        result = min(maximum, result)
    return result


def _sort_arg(value: Any, *, default: str = "cpu") -> str:
    allowed = {"cpu", "memory", "pid", "threads", "name"}
    candidate = str(value or default).strip().lower()
    return candidate if candidate in allowed else default


def get_gpu_stats() -> dict[str, float] | None:
    if not PYNVML_AVAILABLE or _gpu_handle is None:
        return None
    try:
        util = pynvml.nvmlDeviceGetUtilizationRates(_gpu_handle)
        mem_info = pynvml.nvmlDeviceGetMemoryInfo(_gpu_handle)
        temp = pynvml.nvmlDeviceGetTemperature(_gpu_handle, pynvml.NVML_TEMPERATURE_GPU)
        return {
            "util": float(util.gpu),
            "mem_used": mem_info.used / (1024**3),
            "mem_total": mem_info.total / (1024**3),
            "mem_percent": (mem_info.used / mem_info.total) * 100,
            "temp": float(temp),
        }
    except Exception:
        return None


@app.route("/")
def index():
    return send_file(HTML_DIR / "dashboard.html")


@app.route("/api/health")
def health():
    return jsonify({"status": "ok", "surface": "web", "port": DEFAULT_PORT})


@app.route("/api/stats")
def stats():
    try:
        snapshot = collector.sample()
    except Exception:
        return jsonify({"error": "collection failed"}), 500

    stats_data: dict[str, Any] = {
        "cpu": snapshot.cpu.total_percent,
        "mem": snapshot.memory.percent,
        "disk": snapshot.disks[0].percent if snapshot.disks else 0,
        "temp": snapshot.temperature_c or 0,
        "net_up": snapshot.network.upload_bps,
        "net_down": snapshot.network.download_bps,
        "uptime": snapshot.uptime_seconds,
        "load": snapshot.cpu.load_avg_1m,
        "processes": snapshot.process_count,
        "threads": snapshot.thread_count,
        "cpu_freq": snapshot.cpu.frequency_mhz / 1000 if snapshot.cpu.frequency_mhz else None,
        "cpu_cores": snapshot.cpu.core_count,
    }
    gpu = get_gpu_stats()
    if gpu:
        stats_data.update(
            {
                "gpu_util": gpu["util"],
                "gpu_temp": gpu["temp"],
                "gpu_mem_percent": gpu["mem_percent"],
                "gpu_mem_used": gpu["mem_used"],
            }
        )
    return jsonify(stats_data)


@app.route("/api/processes")
def processes():
    process_cfg = config.get("process", {})
    rows = process_service.list_processes(
        include_kernel=_bool_arg(request.args.get("include_kernel"), default=bool(process_cfg.get("show_kernel", False))),
        sort_by=_sort_arg(request.args.get("sort"), default="cpu"),
        descending=not _bool_arg(request.args.get("ascending"), default=False),
        query=str(request.args.get("query", "")).strip(),
        limit=_int_arg(request.args.get("limit"), default=int(process_cfg.get("max_display", 15)), minimum=1, maximum=500),
        username=str(request.args.get("username", "")).strip() or None,
    )
    return jsonify(
        [
            {
                "pid": row.pid,
                "name": row.name,
                "cpu": row.cpu_percent,
                "mem": row.memory_percent,
                "threads": row.threads,
                "status": row.status,
                "username": row.username,
            }
            for row in rows
        ]
    )


@app.route("/api/process/<int:pid>")
def process_detail(pid: int):
    detail = process_service.get_process_detail(pid)
    status = 200 if "error" not in detail else 404
    return jsonify(detail), status


@app.route("/api/process/<int:pid>/kill", methods=["POST"])
def kill_process(pid: int):
    return _run_process_action(lambda: process_service.terminate_process(pid))


@app.route("/api/process/<int:pid>/stop", methods=["POST"])
def stop_process(pid: int):
    return _run_process_action(lambda: process_service.suspend_process(pid))


@app.route("/api/process/<int:pid>/resume", methods=["POST"])
def resume_process(pid: int):
    return _run_process_action(lambda: process_service.resume_process(pid))


def _run_process_action(action):
    import psutil

    try:
        return jsonify(action())
    except psutil.NoSuchProcess:
        return jsonify({"success": False, "message": "Process not found"}), 404
    except psutil.AccessDenied:
        return jsonify({"success": False, "message": "Access denied"}), 403


if __name__ == "__main__":
    print(
        f"""
    ╔═══════════════════════════════════════════╗
    ║                                           ║
    ║     ◈ VANTA MONITOR - Web Dashboard ◈    ║
    ║                                           ║
    ║     Open: http://localhost:{DEFAULT_PORT}           ║
    ║                                           ║
    ╚═══════════════════════════════════════════╝
    """
    )
    app.run(host="0.0.0.0", port=DEFAULT_PORT, debug=False, threaded=True)
