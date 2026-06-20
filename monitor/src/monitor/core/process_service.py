import psutil
from monitor.core.models import ProcessRow
from typing import Optional


# Kernel process name prefixes and substrings — used to filter OS threads
# that are not useful to display to a user.
KERNEL_PREFIXES: tuple[str, ...] = (
    "kworker",
    "ksoftirqd",
    "migration",
    "rcu",
    "watchdog",
    "irq/",
    "nv_queue",
    "nv_open_q",
    "uvm",
    "kthreadd",
    "kdevtmpfs",
    "kprobe",
    "khungtaskd",
    "oom_reaper",
    "writeback",
    "ksmd",
    "khugepaged",
    "crypto",
    "kintegrityd",
    "bioset",
    "xen-",
    "xenbus",
    "xenwatch",
    "degcd",
    "deferwq",
    "charger_manager",
    "kaluad",
    "kmpath",
    "ipv6_addrconf",
    "acpi_thermal",
    "ata_sff",
    "scsi_",
    "fuse",
    "devfreq",
    "pool_workqueue",
    "idle_inject",
    "cpuhp",
    "mm_percpu",
    "rcu_",
    "perf",
    "migration_",
    "posix_cgroup",
    "blkcg",
    "kauditd",
    "kcompactd",
    "ksgxd",
    "kswapd",
    "hwrng",
    "card",
    "psimon",
    "drm",
    "i915",
    "irq",
    "rcuc",
    "rcub",
    "kstrp",
    "nv_queue_uvm",
    "nvidia-worker",
)


def looks_like_kernel(name_lower: str) -> bool:
    """Heuristic: kernel threads start with a known prefix or contain '/'."""
    if "/" in name_lower:
        return True
    if name_lower.startswith(KERNEL_PREFIXES):
        return True
    return False


def sort_key_for(column: str) -> callable:
    """Return a key function for a column name (cpu->cpu_percent, etc)."""
    col = column.strip().lower().replace("-", "_")
    ATTR_MAP: dict[str, str] = {
        "cpu": "cpu_percent",
        "mem": "memory_percent",
        "memory": "memory_percent",
        "pid": "pid",
        "threads": "threads",
        "name": "name",
        "status": "status",
    }
    attr = ATTR_MAP.get(col, f"{col}_percent" if col not in ("pid", "threads", "name", "status") else col)

    def keyfn(row: ProcessRow) -> float | str:
        val = getattr(row, attr, row.cpu_percent)
        if isinstance(val, float | int):
            return val
        return str(val)

    return keyfn


def next_sort_column(current: str) -> str:
    columns = ["cpu", "memory", "pid", "threads", "name"]
    normalized = current.strip().lower()
    if normalized not in columns:
        return columns[0]
    idx = columns.index(normalized)
    return columns[(idx + 1) % len(columns)]


class ProcessService:
    def list_processes(
        self,
        *,
        include_kernel: bool = False,
        sort_by: str = "cpu",
        descending: bool = True,
        query: str = "",
        limit: int = 50,
    ) -> list[ProcessRow]:
        rows: list[ProcessRow] = []
        query_lower = query.lower().strip()
        for proc in psutil.process_iter(["pid", "name", "cpu_percent", "memory_percent", "status", "num_threads", "username"]):
            try:
                info = proc.info
                name = (info.get("name") or "").strip()
                name_lower = name.lower()
                if not include_kernel and looks_like_kernel(name_lower):
                    continue
                if query_lower and query_lower not in name_lower and query_lower not in str(info["pid"]):
                    continue
                rows.append(
                    ProcessRow(
                        pid=info["pid"],
                        name=name,
                        cpu_percent=float(info.get("cpu_percent") or 0.0),
                        memory_percent=float(info.get("memory_percent") or 0.0),
                        status=str(info.get("status") or "unknown"),
                        threads=int(info.get("num_threads") or 0),
                        username=info.get("username"),
                    )
                )
            except (psutil.NoSuchProcess, psutil.AccessDenied):
                continue
        keyfn = sort_key_for(sort_by)
        rows.sort(key=keyfn, reverse=descending)
        return rows[:limit]

    def terminate_process(self, pid: int) -> dict:
        """Kill a process. Returns {'success': True} or raises on access."""
        p = psutil.Process(pid)
        p.terminate()
        return {"success": True, "message": f"Terminated PID {pid}"}

    def suspend_process(self, pid: int) -> dict:
        p = psutil.Process(pid)
        p.suspend()
        return {"success": True, "message": f"Suspended PID {pid}"}

    def resume_process(self, pid: int) -> dict:
        p = psutil.Process(pid)
        p.resume()
        return {"success": True, "message": f"Resumed PID {pid}"}

