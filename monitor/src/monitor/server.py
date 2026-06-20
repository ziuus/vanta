#!/usr/bin/env python3
"""Vanta Monitor Web Dashboard — Flask API server."""

import json
import time
from pathlib import Path

from flask import Flask, send_file, jsonify, request
from flask_cors import CORS

from monitor.core.collectors import SystemCollector

try:
    import pynvml
    PYNVML_AVAILABLE = True
except ImportError:
    PYNVML_AVAILABLE = False

# Initialize pynvml
_gpu_handle = None
if PYNVML_AVAILABLE:
    try:
        pynvml.nvmlInit()
        _gpu_handle = pynvml.nvmlDeviceGetHandleByIndex(0)
    except Exception:
        PYNVML_AVAILABLE = False


DEFAULT_PORT = 5001


def get_gpu_stats():
    """Get GPU utilization, memory, and temperature."""
    if not PYNVML_AVAILABLE or _gpu_handle is None:
        return None
    try:
        util = pynvml.nvmlDeviceGetUtilizationRates(_gpu_handle)
        mem_info = pynvml.nvmlDeviceGetMemoryInfo(_gpu_handle)
        temp = pynvml.nvmlDeviceGetTemperature(_gpu_handle, pynvml.NVML_TEMPERATURE_GPU)
        return {
            "util": util.gpu,
            "mem_used": mem_info.used / (1024**3),
            "mem_total": mem_info.total / (1024**3),
            "mem_percent": (mem_info.used / mem_info.total) * 100,
            "temp": temp,
        }
    except Exception:
        return None


app = Flask(__name__)
CORS(app)

# Config path points to vanta/monitor/config.json
CONFIG_PATH = Path(__file__).parent.parent.parent / "config.json"


def load_config():
    if CONFIG_PATH.exists():
        with open(CONFIG_PATH) as f:
            return json.load(f)
    return {
        "video": {"path": "", "auto_play": True},
        "process": {"show_kernel": False, "max_display": 15, "auto_refresh": True},
        "audio": {"simulated": True, "sensitivity": 0.5},
        "ui": {"refresh_rate": 0.5, "theme": "dark"},
    }


config = load_config()
collector = SystemCollector()

HTML_DIR = Path(__file__).parent


@app.route("/")
def index():
    return send_file(HTML_DIR / "dashboard.html")


@app.route("/api/stats")
def stats():
    try:
        snapshot = collector.sample()
    except Exception:
        return jsonify({"error": "collection failed"}), 500

    stats_data = {
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

    # Add GPU stats if available
    gpu = get_gpu_stats()
    if gpu:
        stats_data["gpu_util"] = gpu["util"]
        stats_data["gpu_temp"] = gpu["temp"]
        stats_data["gpu_mem_percent"] = gpu["mem_percent"]
        stats_data["gpu_mem_used"] = gpu["mem_used"]

    return jsonify(stats_data)


@app.route("/api/processes")
def processes():
    show_kernel = request.args.get("include_kernel", config.get("process", {}).get("show_kernel", False))
    if isinstance(show_kernel, str):
        show_kernel = show_kernel.lower() in ("true", "1", "yes")
    max_display = request.args.get("limit", config.get("process", {}).get("max_display", 15))
    if isinstance(max_display, str):
        max_display = int(max_display)
    sort_by = request.args.get("sort", "cpu")
    query = request.args.get("query", "")

    from monitor.core.process_service import ProcessService
    svc = ProcessService()
    rows = svc.list_processes(
        include_kernel=show_kernel,
        sort_by=sort_by,
        query=query,
        limit=int(max_display),
    )
    return jsonify([
        {"pid": r.pid, "name": r.name, "cpu": r.cpu_percent,
         "mem": r.memory_percent, "threads": r.threads, "status": r.status,
         "username": r.username}
        for r in rows
    ])


@app.route("/api/process/<int:pid>/kill", methods=["POST"])
def kill_process(pid):
    import psutil
    try:
        p = psutil.Process(pid)
        p.kill()
        return jsonify({"success": True, "message": f"Killed PID {pid}"})
    except psutil.NoSuchProcess:
        return jsonify({"success": False, "message": "Process not found"}), 404
    except psutil.AccessDenied:
        return jsonify({"success": False, "message": "Access denied"}), 403


@app.route("/api/process/<int:pid>/stop", methods=["POST"])
def stop_process(pid):
    import psutil
    try:
        p = psutil.Process(pid)
        p.suspend()
        return jsonify({"success": True, "message": f"Stopped PID {pid}"})
    except psutil.NoSuchProcess:
        return jsonify({"success": False, "message": "Process not found"}), 404
    except psutil.AccessDenied:
        return jsonify({"success": False, "message": "Access denied"}), 403


@app.route("/api/process/<int:pid>/resume", methods=["POST"])
def resume_process(pid):
    import psutil
    try:
        p = psutil.Process(pid)
        p.resume()
        return jsonify({"success": True, "message": f"Resumed PID {pid}"})
    except psutil.NoSuchProcess:
        return jsonify({"success": False, "message": "Process not found"}), 404
    except psutil.AccessDenied:
        return jsonify({"success": False, "message": "Access denied"}), 403


if __name__ == "__main__":
    print(f"""
    ╔═══════════════════════════════════════════╗
    ║                                           ║
    ║     ◈ VANTA MONITOR - Web Dashboard ◈    ║
    ║                                           ║
    ║     Open: http://localhost:{DEFAULT_PORT}           ║
    ║                                           ║
    ╚═══════════════════════════════════════════╝
    """)
    app.run(host="0.0.0.0", port=DEFAULT_PORT, debug=False, threaded=True)
