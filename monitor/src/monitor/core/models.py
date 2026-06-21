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
    device: str = ""
    mountpoint: str = ""
    percent: float = 0.0
    used_bytes: int = 0
    free_bytes: int = 0
    total_bytes: int = 0
    io_read_bps: float = 0.0
    io_write_bps: float = 0.0
    io_busy_percent: float = 0.0


@dataclass(slots=True)
class IfaceSnapshot:
    name: str
    upload_bps: float = 0.0
    download_bps: float = 0.0
    bytes_sent: int = 0
    bytes_recv: int = 0
    is_up: bool = False
    speed: int = 0


@dataclass(slots=True)
class NetworkSnapshot:
    upload_bps: float
    download_bps: float
    bytes_sent: int
    bytes_recv: int
    interfaces: list[IfaceSnapshot] = field(default_factory=list)


@dataclass(slots=True)
class GpuSnapshot:
    util_percent: float
    temperature_c: float
    memory_percent: float
    memory_used_bytes: int
    memory_total_bytes: int
    memory_util_percent: float = 0.0
    power_watts: float = 0.0
    power_max_watts: float = 0.0
    clock_graphics_mhz: int = 0
    clock_mem_mhz: int = 0
    encoder_util_percent: float = 0.0
    decoder_util_percent: float = 0.0
    pcie_tx_bps: float = 0.0
    pcie_rx_bps: float = 0.0
    name: str = ""


@dataclass(slots=True)
class BatterySnapshot:
    percent: float
    status: str  # Charging, Discharging, Full, Unknown, Not charging
    power_watts: float = 0.0
    time_to_empty_min: float | None = None
    time_to_full_min: float | None = None


@dataclass(slots=True)
class ProcessRow:
    pid: int
    name: str
    cpu_percent: float
    memory_percent: float
    status: str
    threads: int
    username: str | None
    ppid: int = 0  # parent PID for tree views


@dataclass(slots=True)
class SystemSnapshot:
    cpu: CpuSnapshot
    memory: MemorySnapshot
    disks: list[DiskSnapshot]
    network: NetworkSnapshot
    gpu: Optional[GpuSnapshot]
    process_count: int
    thread_count: int
    uptime_seconds: float
    temperature_c: float | None
    battery: Optional[BatterySnapshot] = None
    cpu_power_watts: float = 0.0
