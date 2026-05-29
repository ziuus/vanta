import os
import psutil
import time
import random
import calendar
import json
import threading
from dataclasses import dataclass, field
from datetime import datetime
from pathlib import Path

try:
    import pynvml
    PYNVML_AVAILABLE = True
except ImportError:
    PYNVML_AVAILABLE = False


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

# Use refresh_rate from config (default 0.5 seconds)
_REFRESH_RATE = config.get("ui", {}).get("refresh_rate", 0.5)
_MAX_DISPLAY = config.get("process", {}).get("max_display", 15)

# Initialize pynvml if available
_gpu_handle = None
if PYNVML_AVAILABLE:
    try:
        pynvml.nvmlInit()
        _gpu_handle = pynvml.nvmlDeviceGetHandleByIndex(0)
    except Exception:
        PYNVML_AVAILABLE = False


def get_gpu_stats():
    """Get GPU utilization, memory, and temperature."""
    if PYNVML_AVAILABLE and _gpu_handle is not None:
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
            pass
            
    # Fallback to nvidia-smi
    try:
        import subprocess
        result = subprocess.run(
            ["nvidia-smi", "--query-gpu=utilization.gpu,memory.used,memory.total,temperature.gpu", "--format=csv,noheader,nounits"],
            capture_output=True, text=True, timeout=1
        )
        if result.returncode == 0:
            parts = result.stdout.strip().split(', ')
            if len(parts) == 4:
                util, mem_used, mem_total, temp = [float(x) for x in parts]
                return {
                    "util": util,
                    "mem_used": mem_used / 1024,
                    "mem_total": mem_total / 1024,
                    "mem_percent": (mem_used / mem_total) * 100 if mem_total > 0 else 0,
                    "temp": temp,
                }
    except Exception:
        pass
        
    return None


def format_bytes(bps: float) -> str:
    for unit in ["B", "KB", "MB", "GB"]:
        if bps < 1024:
            return f"{bps:.1f}{unit}"
        bps /= 1024
    return f"{bps:.1f}TB"


def format_uptime(seconds: float) -> str:
    days = int(seconds // 86400)
    hours = int((seconds % 86400) // 3600)
    mins = int((seconds % 3600) // 60)
    if days > 0:
        return f"{days}d {hours}h"
    return f"{hours}h {mins}m"


@dataclass
class SystemStats:
    cpu_history: list = field(default_factory=lambda: [0.0] * 40)
    mem_history: list = field(default_factory=lambda: [0.0] * 40)
    prev_net: tuple = field(default_factory=lambda: (0, 0))
    prev_net_time: float = field(default_factory=time.time)
    boot_time: float = field(default_factory=psutil.boot_time)
    process_list: list = field(default_factory=list)
    selected_proc: int = 0


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


class AudioVisualizer:
    def __init__(self):
        self.bars = [0.0] * 32
        self.target_bars = [0.0] * 32
        self._running = False
        self.sensitivity = config.get("audio", {}).get("sensitivity", 0.5)

    def start(self):
        self._running = True

        def update():
            while self._running:
                self.target_bars = [
                    random.uniform(20 * self.sensitivity, 100 * self.sensitivity)
                    if random.random() > 0.15
                    else random.uniform(5, 30 * self.sensitivity)
                    for _ in range(32)
                ]
                time.sleep(0.04)
                for i in range(len(self.bars)):
                    self.bars[i] += (self.target_bars[i] - self.bars[i]) * 0.25

        threading.Thread(target=update, daemon=True).start()

    def stop(self):
        self._running = False

    def render(self):
        chars = " ▁▂▃▄▅▆▇█"
        colors = ["#4ade80", "#22d3ee", "#818cf8", "#c084fc", "#f472b6"]
        return "".join(
            f"[{colors[min(int(v / 20), 4)]}]{chars[min(int(v / 12.5), 8)]}[/]"
            for v in self.bars
        )


# Import textual after defining helpers
from textual.app import App, ComposeResult
from textual.containers import Container, Horizontal
from textual.widgets import Static, Sparkline, Header, Footer, Button, ListView, ListItem
from textual.reactive import reactive
from textual.binding import Binding

from rich.text import Text
import subprocess

class FastfetchDisplay(Static):
    def on_mount(self):
        self.set_interval(60.0, self.update_fastfetch)
        self.update_fastfetch()

    def update_fastfetch(self):
        try:
            result = subprocess.run(["fastfetch", "--logo", "none", "--pipe", "false"], capture_output=True, text=True)
            if result.returncode == 0:
                self.update(Text.from_ansi(result.stdout.strip()))
            else:
                self.update(f"Fastfetch error: {result.stderr}")
        except Exception as e:
            self.update(f"Fastfetch error: {e}")

class MatrixRain(Static):
    def on_mount(self):
        self.chars = "ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789@#$%^&*"
        self.columns = []
        self.set_interval(0.1, self.tick)
        
    def on_resize(self, event):
        self.width = event.size.width
        self.height = event.size.height
        self.columns = [{"pos": random.randint(-self.height, 0), "speed": random.randint(1, 2)} for _ in range(self.width)]
        
    def tick(self):
        if not self.columns or self.height == 0:
            return
        
        text = Text()
        for y in range(self.height):
            for col in self.columns:
                char = random.choice(self.chars)
                if 0 <= y - col["pos"] < 1:
                    text.append(char, style="bold white")
                elif 1 <= y - col["pos"] < 6:
                    text.append(char, style="bold #4ade80")
                elif 6 <= y - col["pos"] < 12:
                    text.append(char, style="#22c55e")
                else:
                    text.append(" ")
            if y < self.height - 1:
                text.append("\n")
        
        for col in self.columns:
            col["pos"] += col["speed"]
            if col["pos"] > self.height + 12:
                col["pos"] = random.randint(-self.height, 0)
                
        self.update(text)




class VantaMonitorTUI(App):
    CSS = """
    Screen { background: #0a0a0f; }

    Header { background: #0a0a0f; border: none; }
    Header .title { color: #06b6d4; }

    Footer { background: #0a0a0f; height: 1; }

    #main {
        layout: grid;
        grid-size: 4 3;
        grid-gutter: 1 1;
        padding: 0;
        height: 100%;
    }

    .panel {
        background: #0f0f1a;
        border: solid #1e1e3f;
        padding: 1;
    }

    .panel:hover { border: solid #2e2e5f; }

    .panel-title {
        color: #64748b;
        text-style: bold;
    }

    #sysinfo-panel { row-span: 2; overflow-y: hidden; }
    #matrix-panel { padding: 0; overflow: hidden; border: none; }
    #matrix-panel:hover { border: none; }

    .metric-value { color: #06b6d4; text-style: bold; }

    #process-panel { row-span: 3; }

    #process-list {
        height: 100%;
        background: transparent;
    }

    #process-list .textual-list-view--cursor {
        background: #1e1e3f;
    }

    Button { margin: 0 1; }
    """

    stats = reactive(SystemStats())
    _audio: AudioVisualizer = None

    BINDINGS = [
        Binding("q", "quit", "Quit", show=True),
        Binding("r", "refresh", "Refresh", show=True),
        Binding("j", "proc_down", "Down", show=True),
        Binding("k", "proc_up", "Up", show=True),
        Binding("x", "kill_selected", "Kill", show=True),
    ]

    def compose(self) -> ComposeResult:
        yield Header()

        with Container(id="main"):
            with Container(classes="panel", id="clock-panel"):
                yield Static("◈  VANTA MONITOR", classes="panel-title")
                yield Static("00:00:00", id="time-display", classes="time-display")
                yield Static("Monday", id="day-display", classes="day-display")
                yield Static("01 Jan 2026", id="date-display", classes="date-display")

            with Container(classes="panel"):
                yield Static("◈  CALENDAR", classes="panel-title")
                yield Static("", id="calendar-display")

            with Container(classes="panel"):
                yield Static("◈  CPU", classes="panel-title")
                yield Static("--%", id="cpu-value", classes="metric-value")
                yield Static("Cores: --  |  Freq: -- GHz", id="cpu-info", classes="date-display")
                yield Sparkline([], id="cpu-spark")

            with Container(classes="panel"):
                yield Static("◈  MEMORY", classes="panel-title")
                yield Static("--%", id="mem-value", classes="metric-value")
                yield Static("-- GB / -- GB", id="mem-info", classes="date-display")
                yield Sparkline([], id="mem-spark")

            with Container(classes="panel"):
                yield Static("◈  GPU", classes="panel-title")
                yield Static("--%", id="gpu-value", classes="metric-value")
                yield Static("Temp: --C  |  VRAM: --%", id="gpu-info", classes="date-display")

            with Container(classes="panel"):
                yield Static("◈  DISK", classes="panel-title")
                yield Static("--%", id="disk-value", classes="metric-value")
                yield Static("/  -- GB free", id="disk-info", classes="date-display")

            with Container(classes="panel"):
                yield Static("◈  NETWORK", classes="panel-title")
                yield Static("^ -- KB/s", id="net-up", classes="metric-value")
                yield Static("v -- KB/s", id="net-down", classes="metric-value")

            with Container(classes="panel"):
                yield Static("◈  AUDIO", classes="panel-title")
                yield Static("", id="audio-viz")

            with Container(classes="panel", id="process-panel"):
                yield Static("◈  PROCESS MANAGER", classes="panel-title")
                yield ListView(id="process-list")
                with Horizontal():
                    yield Button("^", id="btn-up", variant="primary")
                    yield Button("v", id="btn-down", variant="primary")
                    yield Button("KILL", id="btn-kill", variant="error")

            with Container(classes="panel"):
                yield Static("◈  SYSTEM", classes="panel-title")
                yield Static("Processes: --", id="stat-procs")
                yield Static("Threads: --", id="stat-threads")
                yield Static("Uptime: --", id="stat-uptime")
                yield Static("Load: --", id="stat-load")
                yield Static("Temp: --C", id="stat-temp")

        yield Footer()

    def on_mount(self) -> None:
        self.title = "◈ Vanta Monitor ◈"
        self.subtitle = "System Command Center"

        self._audio = AudioVisualizer()
        self._audio.start()

        # Use refresh rate from config
        self.set_interval(_REFRESH_RATE, self.update_stats)
        self.set_interval(0.1, self.refresh_audio)
        self.set_interval(5.0, self.update_processes)

    def on_unmount(self) -> None:
        if self._audio:
            self._audio.stop()
        if PYNVML_AVAILABLE:
            try:
                pynvml.nvmlShutdown()
            except Exception:
                pass

    def refresh_audio(self):
        try:
            if self._audio:
                self.query_one("#audio-viz", Static).update(self._audio.render())
        except Exception:
            pass

    def update_stats(self):
        try:
            cpu = psutil.cpu_percent(interval=0.1)
            cpu_cores = psutil.cpu_count()
            cpu_freq = psutil.cpu_freq().current / 1000 if psutil.cpu_freq() else 0

            mem = psutil.virtual_memory()
            mem_used = mem.used / (1024**3)
            mem_total = mem.total / (1024**3)

            disk = psutil.disk_usage("/")

            # Network with proper rate calculation
            now = time.time()
            net = psutil.net_io_counters()
            elapsed = now - self.stats.prev_net_time

            if elapsed > 0:
                up = (net.bytes_sent - self.stats.prev_net[0]) / elapsed
                down = (net.bytes_recv - self.stats.prev_net[1]) / elapsed
            else:
                up, down = 0, 0

            self.stats.prev_net = (net.bytes_sent, net.bytes_recv)
            self.stats.prev_net_time = now

            cpu_hist = self.stats.cpu_history[1:] + [cpu]
            mem_hist = self.stats.mem_history[1:] + [mem.percent]
            self.stats.cpu_history = cpu_hist
            self.stats.mem_history = mem_hist

            self.query_one("#cpu-value", Static).update(f"{cpu:.1f}%")
            self.query_one("#cpu-info", Static).update(
                f"Cores: {cpu_cores}  |  {cpu_freq:.2f} GHz"
            )
            self.query_one("#cpu-spark", Sparkline).data = cpu_hist

            self.query_one("#mem-value", Static).update(f"{mem.percent:.1f}%")
            self.query_one("#mem-info", Static).update(
                f"{mem_used:.1f} GB / {mem_total:.1f} GB"
            )
            self.query_one("#mem-spark", Sparkline).data = mem_hist

            self.query_one("#disk-value", Static).update(f"{disk.percent:.1f}%")
            self.query_one("#disk-info", Static).update(
                f"/  {disk.free / (1024**3):.1f} GB free"
            )

            self.query_one("#net-up", Static).update(f"^ {format_bytes(up)}/s")
            self.query_one("#net-down", Static).update(f"v {format_bytes(down)}/s")

            uptime = time.time() - self.stats.boot_time
            load = os.getloadavg()[0] if hasattr(os, "getloadavg") else 0

            self.query_one("#stat-procs", Static).update(f"Processes: {len(psutil.pids())}")
            self.query_one("#stat-threads", Static).update(f"Threads: {threading.active_count()}")
            self.query_one("#stat-uptime", Static).update(f"Uptime: {format_uptime(uptime)}")
            self.query_one("#stat-load", Static).update(f"Load: {load:.2f}")

            try:
                temps = psutil.sensors_temperatures()
                for entries in temps.values():
                    if entries and entries[0].current:
                        self.query_one("#stat-temp", Static).update(
                            f"Temp: {entries[0].current:.0f}C"
                        )
                        break
            except Exception:
                pass

            # GPU stats
            gpu = get_gpu_stats()
            if gpu:
                self.query_one("#gpu-value", Static).update(f"{gpu['util']}%")
                self.query_one("#gpu-info", Static).update(
                    f"Temp: {gpu['temp']}C  |  VRAM: {gpu['mem_percent']:.0f}%"
                )
            else:
                self.query_one("#gpu-value", Static).update("N/A")
                self.query_one("#gpu-info", Static).update("No GPU detected")

        except Exception:
            pass

    def update_processes(self):
        try:
            procs = []
            for p in psutil.process_iter(["pid", "name", "cpu_percent", "memory_percent"]):
                try:
                    info = p.info
                    name_lower = info["name"].lower()
                    if "/" in info["name"]:
                        continue
                    if any(kp in name_lower for kp in KERNEL_PROCS):
                        continue
                    procs.append({
                        "pid": info["pid"],
                        "name": info["name"][:20],
                        "cpu": info["cpu_percent"] or 0,
                        "mem": info["memory_percent"] or 0,
                    })
                except Exception:
                    pass

            procs.sort(key=lambda x: x["cpu"], reverse=True)

            # Track selection within bounds
            max_select = min(len(procs), 10) - 1
            if self.stats.selected_proc > max_select:
                self.stats.selected_proc = max(0, max_select)

            self.stats.process_list = procs[:_MAX_DISPLAY]

            list_view = self.query_one("#process-list", ListView)
            items = []
            for i, p in enumerate(procs[:_MAX_DISPLAY]):
                cpu_color = "#4ade80" if p["cpu"] < 30 else "#eab308" if p["cpu"] < 70 else "#ef4444"
                label = f"[{cpu_color}]{p['cpu']:5.1f}%[/] [#06b6d4]{p['name']:<20}[/] [#64748b]{p['mem']:5.1f}%[/]"
                items.append(ListItem(Static(label), id=str(p["pid"])))

            list_view.clear()
            for item in items:
                list_view.append(item)

            # Restore cursor position
            if items and self.stats.selected_proc < len(items):
                list_view.index = self.stats.selected_proc

        except Exception:
            pass

    def action_refresh(self):
        self.update_stats()
        self.update_processes()
        self.notify("Refreshed!", severity="information")

    def action_proc_up(self):
        list_view = self.query_one("#process-list", ListView)
        if self.stats.selected_proc > 0:
            self.stats.selected_proc -= 1
            list_view.index = self.stats.selected_proc

    def action_proc_down(self):
        list_view = self.query_one("#process-list", ListView)
        if self.stats.selected_proc < len(self.stats.process_list) - 1:
            self.stats.selected_proc += 1
            list_view.index = self.stats.selected_proc

    def action_kill_selected(self):
        list_view = self.query_one("#process-list", ListView)
        try:
            selected_id = int(list_view.index)
            if selected_id < len(self.stats.process_list):
                proc = self.stats.process_list[selected_id]
                p = psutil.Process(proc["pid"])
                p.kill()
                self.notify(f"Killed {proc['name']}", severity="warning")
                self.update_processes()
        except psutil.NoSuchProcess:
            self.notify("Process already gone", severity="error")
        except psutil.AccessDenied:
            self.notify("Access denied", severity="error")
        except Exception as e:
            self.notify(f"Error: {e}", severity="error")

    def on_button_pressed(self, event: Button.Pressed):
        btn_id = event.button.id
        if btn_id == "btn-up":
            self.action_proc_up()
        elif btn_id == "btn-down":
            self.action_proc_down()
        elif btn_id == "btn-kill":
            self.action_kill_selected()


def main():
    app = VantaMonitorTUI()
    app.run()


if __name__ == "__main__":
    main()
