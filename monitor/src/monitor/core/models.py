from dataclasses import dataclass, field
from typing import List, Optional


@dataclass(slots=True)
class CpuSnapshot:
    total_percent: float
    per_core_percent: List[float]
    load_avg_1m: float
    frequency_mhz: float | None
    core_count: int


@dataclass(slots=True)
class MemorySnapshot:
    percent: float
    used_bytes: int
    total_bytes: int
    available_bytes: int
    swap_percent: float


@dataclass(slots=True)
class DiskSnapshot:
    mountpoint: str
    percent: float
    used_bytes: int
    free_bytes: int
    total_bytes: int
    io_read_bps: float = 0.0
    io_write_bps: float = 0.0


@dataclass(slots=True)
class NetworkSnapshot:
    upload_bps: float
    download_bps: float
    bytes_sent: int
    bytes_recv: int


@dataclass(slots=True)
class GpuSnapshot:
    util_percent: float
    temperature_c: float
    memory_percent: float
    memory_used_bytes: int
    memory_total_bytes: int


@dataclass(slots=True)
class ProcessRow:
    pid: int
    name: str
    cpu_percent: float
    memory_percent: float
    status: str
    threads: int
    username: str | None


@dataclass(slots=True)
class SystemSnapshot:
    cpu: CpuSnapshot
    memory: MemorySnapshot
    disks: List[DiskSnapshot]
    network: NetworkSnapshot
    gpu: Optional[GpuSnapshot]
    process_count: int
    thread_count: int
    uptime_seconds: float
    temperature_c: float | None
