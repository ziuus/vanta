import os
import time
import psutil
from monitor.core.models import (
    CpuSnapshot,
    DiskSnapshot,
    GpuSnapshot,
    MemorySnapshot,
    NetworkSnapshot,
    SystemSnapshot,
)

try:
    import pynvml

    pynvml.nvmlInit()
    _GPU_HANDLE = pynvml.nvmlDeviceGetHandleByIndex(0)
    _PYNVML_AVAIL = True
except Exception:
    pynvml = None  # type: ignore[assignment]
    _GPU_HANDLE = None
    _PYNVML_AVAIL = False


class SystemCollector:
    def __init__(self) -> None:
        counters = psutil.net_io_counters()
        self._prev_sent = counters.bytes_sent
        self._prev_recv = counters.bytes_recv
        self._prev_time = time.time()
        disk_counters = psutil.disk_io_counters(perdisk=True)
        self._prev_disk_io: dict[str, tuple[int, int]] = {
            name: (c.read_bytes, c.write_bytes)
            for name, c in disk_counters.items()
        }
        self._prev_disk_time = time.time()

    def sample(self) -> SystemSnapshot:
        cpu = self._sample_cpu()
        memory = self._sample_memory()
        disks = self._sample_disks()
        network = self._sample_network()
        gpu = self._sample_gpu()
        temp = self._sample_temperature()
        return SystemSnapshot(
            cpu=cpu,
            memory=memory,
            disks=disks,
            network=network,
            gpu=gpu,
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
        rows: list[DiskSnapshot] = []
        for part in psutil.disk_partitions(all=False):
            try:
                usage = psutil.disk_usage(part.mountpoint)
            except PermissionError:
                continue
            rows.append(
                DiskSnapshot(
                    mountpoint=part.mountpoint,
                    percent=usage.percent,
                    used_bytes=usage.used,
                    free_bytes=usage.free,
                    total_bytes=usage.total,
                )
            )
        # Aggregate disk IO rates (total read/write across all physical disks)
        now = time.time()
        elapsed = max(now - self._prev_disk_time, 0.001)
        current_io = psutil.disk_io_counters(perdisk=True)
        total_read = 0.0
        total_write = 0.0
        for name, cur in current_io.items():
            prev = self._prev_disk_io.get(name)
            if prev is not None:
                total_read += (cur.read_bytes - prev[0]) / elapsed
                total_write += (cur.write_bytes - prev[1]) / elapsed
        self._prev_disk_io = {
            name: (c.read_bytes, c.write_bytes)
            for name, c in current_io.items()
        }
        self._prev_disk_time = now
        if rows:
            rows[0].io_read_bps = total_read
            rows[0].io_write_bps = total_write
        return rows

    def _sample_network(self) -> NetworkSnapshot:
        counters = psutil.net_io_counters()
        now = time.time()
        elapsed = max(now - self._prev_time, 0.001)
        upload_bps = (counters.bytes_sent - self._prev_sent) / elapsed
        download_bps = (counters.bytes_recv - self._prev_recv) / elapsed
        self._prev_sent = counters.bytes_sent
        self._prev_recv = counters.bytes_recv
        self._prev_time = now
        return NetworkSnapshot(
            upload_bps=upload_bps,
            download_bps=download_bps,
            bytes_sent=counters.bytes_sent,
            bytes_recv=counters.bytes_recv,
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
            return GpuSnapshot(
                util_percent=float(util.gpu),
                temperature_c=float(temp),
                memory_percent=(mem_info.used / mem_info.total) * 100,
                memory_used_bytes=mem_info.used,
                memory_total_bytes=mem_info.total,
            )
        except Exception:
            return None

    def _safe_threads(self):
        for proc in psutil.process_iter():
            try:
                yield proc.num_threads()
            except Exception:
                continue
