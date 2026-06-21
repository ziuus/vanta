import psutil
import signal
from monitor.core.models import ProcessRow


SECRET_ENV_TOKENS = (
    "key",
    "token",
    "secret",
    "password",
    "passwd",
    "cookie",
    "session",
    "credential",
    "auth",
)


def _sanitize_env_preview(env: dict[str, str], *, limit: int = 8) -> list[str]:
    preview: list[str] = []
    for key in sorted(env)[:limit]:
        value = str(env[key])
        lowered = key.lower()
        if any(token in lowered for token in SECRET_ENV_TOKENS):
            value = "<redacted>"
        elif len(value) > 96:
            value = value[:93] + "..."
        preview.append(f"{key}={value}"[:120])
    return preview


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

# Human-readable signal names
SIGNALS: dict[str, int] = {
    "TERM": signal.SIGTERM,
    "KILL": signal.SIGKILL,
    "STOP": signal.SIGSTOP,
    "CONT": signal.SIGCONT,
    "HUP": signal.SIGHUP,
    "INT": signal.SIGINT,
    "USR1": signal.SIGUSR1,
    "USR2": signal.SIGUSR2,
}


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
        return str(val).lower()

    return keyfn


SORT_COLUMNS = ["cpu", "memory", "pid", "threads", "name"]


def next_sort_column(current: str) -> str:
    normalized = current.strip().lower()
    if normalized not in SORT_COLUMNS:
        return SORT_COLUMNS[0]
    idx = SORT_COLUMNS.index(normalized)
    return SORT_COLUMNS[(idx + 1) % len(SORT_COLUMNS)]


def prev_sort_column(current: str) -> str:
    normalized = current.strip().lower()
    if normalized not in SORT_COLUMNS:
        return SORT_COLUMNS[0]
    idx = SORT_COLUMNS.index(normalized)
    return SORT_COLUMNS[(idx - 1) % len(SORT_COLUMNS)]


SIGNAL_LIST = ["TERM", "KILL", "STOP", "CONT", "HUP", "INT", "USR1", "USR2"]


class ProcessService:
    def list_processes(
        self,
        *,
        include_kernel: bool = False,
        sort_by: str = "cpu",
        descending: bool = True,
        query: str = "",
        limit: int = 50,
        username: str | None = None,
    ) -> list[ProcessRow]:
        rows: list[ProcessRow] = []
        query_lower = query.lower().strip()
        for proc in psutil.process_iter(
            ["pid", "name", "cpu_percent", "memory_percent", "status", "num_threads", "username", "ppid"]
        ):
            try:
                info = proc.info
                name = (info.get("name") or "").strip()
                name_lower = name.lower()
                if not include_kernel and looks_like_kernel(name_lower):
                    continue
                if query_lower and query_lower not in name_lower and query_lower not in str(info["pid"]):
                    continue
                if username and info.get("username") != username:
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
                        ppid=info.get("ppid") or 0,
                    )
                )
            except (psutil.NoSuchProcess, psutil.AccessDenied):
                continue
        keyfn = sort_key_for(sort_by)
        rows.sort(key=keyfn, reverse=descending)
        return rows[:limit]

    def get_process_detail(self, pid: int) -> dict:
        """Return detailed info about a process."""
        try:
            p = psutil.Process(pid)
            with p.oneshot():
                mem = p.memory_info()
                try:
                    env = p.environ()
                    env_preview = _sanitize_env_preview(env)
                except (psutil.AccessDenied, OSError, AttributeError):
                    env_preview = []
                try:
                    fds = p.num_fds()
                except (psutil.AccessDenied, AttributeError):
                    fds = None
                try:
                    affinity = p.cpu_affinity()
                except (psutil.AccessDenied, AttributeError):
                    affinity = []
                try:
                    connections = len(p.connections())
                except (psutil.AccessDenied, OSError):
                    connections = None
                return {
                    "pid": pid,
                    "name": p.name(),
                    "exe": p.exe(),
                    "cmdline": " ".join(p.cmdline()) if p.cmdline() else "",
                    "cwd": p.cwd(),
                    "username": p.username(),
                    "status": p.status(),
                    "cpu_percent": p.cpu_percent(interval=0.0),
                    "memory_percent": p.memory_percent(),
                    "memory_rss": mem.rss if mem else 0,
                    "memory_vms": mem.vms if mem else 0,
                    "threads": p.num_threads(),
                    "children": len(p.children()),
                    "fds": fds,
                    "connections": connections,
                    "create_time": p.create_time(),
                    "nice": p.nice(),
                    "cpu_affinity": affinity,
                    "environment_preview": env_preview,
                }
        except (psutil.NoSuchProcess, psutil.AccessDenied) as e:
            return {"pid": pid, "name": f"<{e}>", "error": str(e)}

    def send_signal(self, pid: int, sig_name: str) -> dict:
        sig = SIGNALS.get(sig_name.upper(), signal.SIGTERM)
        p = psutil.Process(pid)
        p.send_signal(sig)
        return {"success": True, "signal": sig_name, "message": f"Sent {sig_name} to PID {pid}"}

    def terminate_process(self, pid: int) -> dict:
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
