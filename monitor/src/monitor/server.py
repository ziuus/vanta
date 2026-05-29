#!/usr/bin/env python3
"""Vanta Monitor Web Dashboard — Flask API server."""
import os
import json
import psutil
import time
from pathlib import Path

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


from flask import Flask, send_file, jsonify, request
from flask_cors import CORS

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

KERNEL_PROCS = {
    "systemd", "kthreadd", "kworker", "migration", "rcu", "watchdog",
    "init", "ksoftirqd", "kdevtmpfs", "netns", "khungtaskd", "oom_reaper",
    "writeback", "ksmd", "khugepaged", "crypto", "kintegrityd", "bioset",
    "xen-", "xenbus", "xenwatch", "sed", "degcd", "deferwq", "charger_manager",
    "kaluad", "kmpath", "ipv6_addrconf", "acpi_thermal", "ata_sff", "scsi_",
    "fuse", "devfreq", "pool_workqueue", "idle_inject", "cpuhp", "mm_percpu",
    "rcu_", "perf", "migration_", "posix_cgroup", "blkcg", "kauditd",
    "kcompactd", "ksgxd", "kswapd", "hwrng", "card", "psimon", "drm", "i915",
    "irq", "rcuc", "rcub", "kstrp",
}


class SystemMonitor:
    def __init__(self):
        self.prev_net = psutil.net_io_counters()
        self.prev_time = time.time()
        self.boot_time = psutil.boot_time()

    def get_cpu(self):
        return psutil.cpu_percent(interval=0.1)

    def get_memory(self):
        return psutil.virtual_memory().percent

    def get_disk(self):
        return psutil.disk_usage("/").percent

    def get_temp(self):
        try:
            temps = psutil.sensors_temperatures()
            for name, entries in temps.items():
                for entry in entries:
                    if entry.current:
                        return entry.current
        except:
            pass
        return None

    def get_network(self):
        current = psutil.net_io_counters()
        elapsed = time.time() - self.prev_time

        up_speed = (
            (current.bytes_sent - self.prev_net.bytes_sent) / elapsed
            if elapsed > 0
            else 0
        )
        down_speed = (
            (current.bytes_recv - self.prev_net.bytes_recv) / elapsed
            if elapsed > 0
            else 0
        )

        self.prev_net = current
        self.prev_time = time.time()

        return up_speed, down_speed

    def get_uptime(self):
        return time.time() - self.boot_time

    def get_load(self):
        try:
            return os.getloadavg()[0] if hasattr(os, "getloadavg") else 0
        except:
            return 0

    def get_process_count(self):
        return len(psutil.pids())

    def get_thread_count(self):
        count = 0
        for p in psutil.process_iter():
            try:
                count += p.num_threads()
            except:
                pass
        return count

    def get_processes(self, show_kernel=False, max_display=15):
        procs = []
        for p in psutil.process_iter(
            ["pid", "name", "cpu_percent", "memory_percent", "num_threads", "username"]
        ):
            try:
                info = p.info
                name_lower = info["name"].lower()
                username = info.get("username", "")

                # Skip kernel/user space processes by checking username
                is_system = username in ("root", "Kernel") or "/" in info["name"]

                # Additional pattern matching for kernel processes
                if not show_kernel:
                    is_kernel = False
                    for kp in KERNEL_PROCS:
                        if kp in name_lower:
                            is_kernel = True
                            break
                    # Also skip if name has / (kernel threads like irq/9-acpi)
                    if is_kernel or "/" in info["name"]:
                        continue

                procs.append(
                    {
                        "pid": info["pid"],
                        "name": info["name"],
                        "cpu": info["cpu_percent"] or 0,
                        "mem": info["memory_percent"] or 0,
                        "threads": info["num_threads"] or 1,
                    }
                )
            except (psutil.NoSuchProcess, psutil.AccessDenied):
                pass

        return sorted(procs, key=lambda x: x["cpu"], reverse=True)[:max_display]


monitor = SystemMonitor()

HTML_DIR = Path(__file__).parent


@app.route("/")
def index():
    return send_file(HTML_DIR / "dashboard.html")


@app.route("/api/stats")
def stats():
    net = monitor.get_network()

    stats_data = {
        "cpu": monitor.get_cpu(),
        "mem": monitor.get_memory(),
        "disk": monitor.get_disk(),
        "temp": monitor.get_temp(),
        "net_up": net[0],
        "net_down": net[1],
        "uptime": monitor.get_uptime(),
        "load": monitor.get_load(),
        "processes": monitor.get_process_count(),
        "threads": monitor.get_thread_count(),
    }

    # Add GPU stats if available
    gpu = get_gpu_stats()
    if gpu:
        stats_data["gpu_util"] = gpu["util"]
        stats_data["gpu_temp"] = gpu["temp"]
        stats_data["gpu_mem_percent"] = gpu["mem_percent"]

    return jsonify(stats_data)


@app.route("/api/processes")
def processes():
    show_kernel = config.get("process", {}).get("show_kernel", False)
    max_display = config.get("process", {}).get("max_display", 15)
    return jsonify(monitor.get_processes(show_kernel, max_display))


@app.route("/api/process/<int:pid>/kill", methods=["POST"])
def kill_process(pid):
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
    try:
        p = psutil.Process(pid)
        p.resume()
        return jsonify({"success": True, "message": f"Resumed PID {pid}"})
    except psutil.NoSuchProcess:
        return jsonify({"success": False, "message": "Process not found"}), 404
    except psutil.AccessDenied:
        return jsonify({"success": False, "message": "Access denied"}), 403


if __name__ == "__main__":
    print("""
    ╔═══════════════════════════════════════════╗
    ║                                           ║
    ║     ◈ VANTA MONITOR - Web Dashboard ◈    ║
    ║                                           ║
    ║     Open: http://localhost:5000           ║
    ║                                           ║
    ╚═══════════════════════════════════════════╝
    """)
    app.run(host="0.0.0.0", port=5000, debug=False, threaded=True)
