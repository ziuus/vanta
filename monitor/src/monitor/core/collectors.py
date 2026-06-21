import os
import time
import psutil
from monitor.core.models import (
    BatterySnapshot,
    CpuSnapshot,
    DiskSnapshot,
    GpuSnapshot,
    IfaceSnapshot,
    MemorySnapshot,
    NetworkSnapshot,
    SystemSnapshot,
)

try:
    import pynvml

    pynvml.nvmlInit()
    _GPU_HANDLE = pynvml.nvmlDeviceGetHandleByIndex(0)
    _PYNVML_AVAIL = True
    _GPU_NAME = ""
    if _GPU_HANDLE is not None:
        try:
            buf = pynvml.nvmlDeviceGetName(_GPU_HANDLE)
            _GPU_NAME = buf if isinstance(buf, str) else buf.decode("utf-8", errors="replace")
        except Exception:
            _GPU_NAME = ""
except Exception:
    pynvml = None  # type: ignore[assignment]
    _GPU_HANDLE = None
    _PYNVML_AVAIL = False
    _GPU_NAME = ""


# RAPL paths — energy in microjoules
_RAPL_PATHS = [
    "/sys/class/powercap/intel-rapl:0/energy_uj",
    "/sys/class/powercap/intel-rapl:0/device/energy_uj",
    "/sys/class/powercap/amd_rapl:0/energy_uj",
]


def _read_int(path: str) -> int | None:
    try:
        with open(path) as f:
            return int(f.read().strip())
    except (FileNotFoundError, PermissionError, ValueError, OSError):
        return None


def _read_str(path: str) -> str | None:
    try:
        with open(path) as f:
            return f.read().strip()
    except (FileNotFoundError, PermissionError, OSError):
        return None


_rapl_path: str | None = None
for _rp in _RAPL_PATHS:
    if _read_int(_rp) is not None:
        _rapl_path = _rp
        break


class SystemCollector:
    def __init__(self) -> None:
        counters = psutil.net_io_counters()
        self._prev_sent = counters.bytes_sent
        self._prev_recv = counters.bytes_recv
        self._prev_time = time.time()

        # Per-interface network tracking
        self._prev_iface: dict[str, tuple[int, int]] = {}
        self._prev_iface_time = time.time()
        self._init_per_iface()

        # Per-disk IO tracking
        self._prev_disk_io: dict[str, tuple[int, int, int]] = {}
        self._prev_disk_time = time.time()

        # RAPL power tracking
        self._prev_rapl_uj: int | None = None
        self._prev_rapl_time = time.time()
        if _rapl_path:
            self._prev_rapl_uj = _read_int(_rapl_path)

    def _init_per_iface(self) -> None:
        try:
            pernic = psutil.net_io_counters(pernic=True)
            for name, stats in pernic.items():
                self._prev_iface[name] = (stats.bytes_sent, stats.bytes_recv)
        except Exception:
            pass

    def sample(self) -> SystemSnapshot:
        cpu = self._sample_cpu()
        memory = self._sample_memory()
        disks = self._sample_disks()
        network = self._sample_network()
        gpu = self._sample_gpu()
        temp = self._sample_temperature()
        battery = self._sample_battery()
        cpu_power = self._sample_cpu_power()
        return SystemSnapshot(
            cpu=cpu,
            memory=memory,
            disks=disks,
            network=network,
            gpu=gpu,
            battery=battery,
            cpu_power_watts=cpu_power,
            process_count=len(psutil.pids()),
            thread_count=sum(self._safe_threads()),
            uptime_seconds=time.time() - psutil.boot_time(),
            temperature_c=temp,
        )

    def _sample_cpu(self) -> CpuSnapshot:
        freq = psutil.cpu_freq()
        load_1m = os.getloadavg()[0] if hasattr(os, "getloadavg") else 0.0
        return CpuSnapshot(
            total_percent=psutil.cpu_percent(interval=0.0),
            per_core_percent=psutil.cpu_percent(interval=0.0, percpu=True),
            load_avg_1m=load_1m,
            frequency_mhz=freq.current if freq else None,
            core_count=psutil.cpu_count(logical=True) or 0,
        )

    def _sample_memory(self) -> MemorySnapshot:
        vm = psutil.virtual_memory()
        sm = psutil.swap_memory()
        return MemorySnapshot(
            percent=vm.percent,
            used_bytes=vm.used,
            total_bytes=vm.total,
            available_bytes=vm.available,
            swap_percent=sm.percent,
        )

    def _sample_disks(self) -> list[DiskSnapshot]:
        """Collect per-device disk usage + per-device I/O rates + I/O busy."""
        rows: list[DiskSnapshot] = []
        now = time.time()
        elapsed = max(now - self._prev_disk_time, 0.001)

        # Per-device I/O counters
        current_io = psutil.disk_io_counters(perdisk=True)
        io_by_device: dict[str, tuple[float, float]] = {}
        total_busy_pct: dict[str, float] = {}

        for name, cur in current_io.items():
            prev = self._prev_disk_io.get(name)
            if prev is not None:
                read_bps = (cur.read_bytes - prev[0]) / elapsed
                write_bps = (cur.write_bytes - prev[1]) / elapsed
                busy_delta = (cur.busy_time - prev[2]) if hasattr(cur, 'busy_time') else 0
            else:
                read_bps = 0.0
                write_bps = 0.0
                busy_delta = 0
            io_by_device[name] = (read_bps, write_bps)
            # busy_time is cumulative in milliseconds; convert to percent over elapsed
            busy_pct = min(100.0, (busy_delta / max(elapsed * 1000, 1)) * 100) if hasattr(cur, 'busy_time') else 0.0
            total_busy_pct[name] = busy_pct

        self._prev_disk_io = {
            name: (c.read_bytes, c.write_bytes, getattr(c, 'busy_time', 0))
            for name, c in current_io.items()
        }
        self._prev_disk_time = now

        # Build per-partition rows with per-device I/O
        for part in psutil.disk_partitions(all=False):
            try:
                usage = psutil.disk_usage(part.mountpoint)
            except PermissionError:
                continue
            # Map mountpoint to device name
            dev_name = part.device.split("/")[-1] if part.device else ""
            # Strip partition number for matching (e.g. nvme0n1p1 -> nvme0n1, sda1 -> sda)
            disk_key = "".join(c for c in dev_name if not c.isdigit())
            if not disk_key:
                disk_key = dev_name
            # Try direct match first, then fallback
            io = io_by_device.get(dev_name)
            if io is None:
                io = io_by_device.get(disk_key, (0.0, 0.0))
            busy_pct_use = total_busy_pct.get(dev_name)
            if busy_pct_use is None:
                busy_pct_use = total_busy_pct.get(disk_key, 0.0)

            rows.append(
                DiskSnapshot(
                    device=dev_name,
                    mountpoint=part.mountpoint,
                    percent=usage.percent,
                    used_bytes=usage.used,
                    free_bytes=usage.free,
                    total_bytes=usage.total,
                    io_read_bps=io[0],
                    io_write_bps=io[1],
                    io_busy_percent=busy_pct_use,
                )
            )
        return rows

    def _sample_network(self) -> NetworkSnapshot:
        """Aggregate + per-interface network counters."""
        counters = psutil.net_io_counters()
        now = time.time()
        elapsed = max(now - self._prev_time, 0.001)
        upload_bps = (counters.bytes_sent - self._prev_sent) / elapsed
        download_bps = (counters.bytes_recv - self._prev_recv) / elapsed
        self._prev_sent = counters.bytes_sent
        self._prev_recv = counters.bytes_recv
        self._prev_time = now

        # Per-interface
        iface_elapsed = max(now - self._prev_iface_time, 0.001)
        ifaces: list[IfaceSnapshot] = []
        try:
            pernic = psutil.net_io_counters(pernic=True)
            stats_map = psutil.net_if_stats()
            for name, cur in pernic.items():
                prev = self._prev_iface.get(name)
                if prev is not None:
                    tx = max(0, (cur.bytes_sent - prev[0])) / iface_elapsed
                    rx = max(0, (cur.bytes_recv - prev[1])) / iface_elapsed
                else:
                    tx = 0.0
                    rx = 0.0
                self._prev_iface[name] = (cur.bytes_sent, cur.bytes_recv)
                st = stats_map.get(name)
                ifaces.append(
                    IfaceSnapshot(
                        name=name,
                        upload_bps=tx,
                        download_bps=rx,
                        bytes_sent=cur.bytes_sent,
                        bytes_recv=cur.bytes_recv,
                        is_up=st.isup if st else False,
                        speed=st.speed if st else 0,
                    )
                )
        except Exception:
            pass
        self._prev_iface_time = now

        return NetworkSnapshot(
            upload_bps=upload_bps,
            download_bps=download_bps,
            bytes_sent=counters.bytes_sent,
            bytes_recv=counters.bytes_recv,
            interfaces=ifaces,
        )

    def _sample_temperature(self) -> float | None:
        try:
            temps = psutil.sensors_temperatures()
        except Exception:
            return None
        for entries in temps.values():
            for entry in entries:
                if entry.current is not None:
                    return float(entry.current)
        return None

    def _sample_gpu(self) -> GpuSnapshot | None:
        if not _PYNVML_AVAIL or _GPU_HANDLE is None:
            return None
        try:
            util = pynvml.nvmlDeviceGetUtilizationRates(_GPU_HANDLE)
            mem_info = pynvml.nvmlDeviceGetMemoryInfo(_GPU_HANDLE)
            temp = pynvml.nvmlDeviceGetTemperature(_GPU_HANDLE, pynvml.NVML_TEMPERATURE_GPU)
            snap = GpuSnapshot(
                util_percent=float(util.gpu),
                temperature_c=float(temp),
                memory_percent=(mem_info.used / mem_info.total) * 100 if mem_info.total else 0.0,
                memory_used_bytes=mem_info.used,
                memory_total_bytes=mem_info.total,
                memory_util_percent=float(util.memory),
                name=_GPU_NAME,
            )
            # Power
            try:
                power_mw = pynvml.nvmlDeviceGetPowerUsage(_GPU_HANDLE)  # type: ignore[union-attr]
                snap.power_watts = power_mw / 1000.0
            except Exception:
                pass
            try:
                limit_mw = pynvml.nvmlDeviceGetPowerManagementLimit(_GPU_HANDLE)  # type: ignore[union-attr]
                snap.power_max_watts = limit_mw / 1000.0
            except Exception:
                pass
            # Clocks
            try:
                snap.clock_graphics_mhz = int(pynvml.nvmlDeviceGetClockInfo(_GPU_HANDLE, pynvml.NVML_CLOCK_GRAPHICS))  # type: ignore[union-attr]
            except Exception:
                pass
            try:
                snap.clock_mem_mhz = int(pynvml.nvmlDeviceGetClockInfo(_GPU_HANDLE, pynvml.NVML_CLOCK_MEM))  # type: ignore[union-attr]
            except Exception:
                pass
            # Encoder / Decoder
            try:
                enc, _ = pynvml.nvmlDeviceGetEncoderUtilization(_GPU_HANDLE)  # type: ignore[union-attr]
                snap.encoder_util_percent = float(enc)
            except Exception:
                pass
            try:
                dec, _ = pynvml.nvmlDeviceGetDecoderUtilization(_GPU_HANDLE)  # type: ignore[union-attr]
                snap.decoder_util_percent = float(dec)
            except Exception:
                pass
            # PCIe throughput
            try:
                snap.pcie_tx_bps = float(pynvml.nvmlDeviceGetPcieThroughput(_GPU_HANDLE, pynvml.NVML_PCIE_UTIL_TX_BYTES))  # type: ignore[union-attr]
                snap.pcie_rx_bps = float(pynvml.nvmlDeviceGetPcieThroughput(_GPU_HANDLE, pynvml.NVML_PCIE_UTIL_RX_BYTES))  # type: ignore[union-attr]
            except Exception:
                pass
            return snap
        except Exception:
            return None

    def _sample_battery(self) -> BatterySnapshot | None:
        """Battery via psutil.sensors_battery() — reads /sys/class/power_supply."""
        try:
            bat = psutil.sensors_battery()
        except Exception:
            return None
        if bat is None:
            return None
        status = "Unknown"
        if bat.power_plugged and bat.percent >= 99:
            status = "Full"
        elif bat.power_plugged:
            status = "Charging"
        elif bat.secsleft == -1:
            status = "Not charging"
        else:
            status = "Discharging"
        # Time: psutil returns secsleft as int (-1 = unknown)
        time_to_empty = bat.secsleft / 60.0 if bat.secsleft > 0 else None
        time_to_full = bat.secsleft / 60.0 if status == "Charging" and bat.secsleft > 0 else None
        # Power draw from sysfs directly (psutil doesn't provide it)
        power_watts = self._sample_battery_power()
        return BatterySnapshot(
            percent=bat.percent,
            status=status,
            power_watts=power_watts,
            time_to_empty_min=time_to_empty,
            time_to_full_min=time_to_full,
        )

    def _sample_battery_power(self) -> float:
        """Read battery power in watts from sysfs."""
        try:
            import glob as _glob
            for bat_dir in _glob.glob("/sys/class/power_supply/BAT*"):
                pw = _read_int(f"{bat_dir}/power_now")
                if pw is not None and pw > 0:
                    return pw / 1_000_000.0  # microwatts -> watts
                cur = _read_int(f"{bat_dir}/current_now")
                vol = _read_int(f"{bat_dir}/voltage_now")
                if cur is not None and vol is not None and cur > 0 and vol > 0:
                    return (cur * vol) / 1_000_000_000_000.0  # both in micro -> watts
        except Exception:
            pass
        return 0.0

    def _sample_cpu_power(self) -> float:
        """CPU package power from Intel RAPL MSR (energy_uj delta)."""
        if _rapl_path is None or self._prev_rapl_uj is None:
            return 0.0
        now_uj = _read_int(_rapl_path)
        if now_uj is None:
            return 0.0
        elapsed = max(time.time() - self._prev_rapl_time, 0.001)
        delta_uj = now_uj - self._prev_rapl_uj
        if delta_uj < 0:  # counter wrapped
            delta_uj += 2**63  # 64-bit counter
        watts = (delta_uj / 1_000_000.0) / elapsed  # Joules / second = watts
        self._prev_rapl_uj = now_uj
        self._prev_rapl_time = time.time()
        return max(0.0, watts)

    def _safe_threads(self):
        for proc in psutil.process_iter():
            try:
                yield proc.num_threads()
            except Exception:
                continue
