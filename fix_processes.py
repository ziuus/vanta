import re

content = open("src/monitors/processes.rs").read()

# Add PREV_CPU
struct_def = """struct IoPrev {
    read: u64,
    write: u64,
    time: Instant,
}

struct CpuPrev {
    jiffies: u64,
    total_jiffies: f64,
}

static PREV_CPU: LazyLock<Mutex<HashMap<u32, CpuPrev>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
"""
content = content.replace("struct IoPrev {\n    read: u64,\n    write: u64,\n    time: Instant,\n}", struct_def)

# Fix read_proc_cpu
old_cpu = """fn read_proc_cpu(pid: u32, total_jiffies: f64) -> f64 {
    let path = format!("/proc/{}/stat", pid);
    if let Ok(content) = fs::read_to_string(&path) {
        let parts: Vec<&str> = content.split_whitespace().collect();
        if parts.len() > 21 {
            if let (Ok(utime), Ok(stime)) = (parts[13].parse::<u64>(), parts[14].parse::<u64>()) {
                if total_jiffies > 0.0 {
                    return ((utime + stime) as f64 / total_jiffies) * 100.0;
                }
            }
        }
    }
    0.0
}"""

new_cpu = """fn read_proc_cpu(pid: u32, total_jiffies: f64) -> f64 {
    let path = format!("/proc/{}/stat", pid);
    if let Ok(content) = fs::read_to_string(&path) {
        let parts: Vec<&str> = content.split_whitespace().collect();
        if parts.len() > 21 {
            if let (Ok(utime), Ok(stime)) = (parts[13].parse::<u64>(), parts[14].parse::<u64>()) {
                let proc_jiffies = utime + stime;
                let mut prev_map = PREV_CPU.lock().unwrap();
                let pct = if let Some(prev) = prev_map.get(&pid) {
                    let d_proc = proc_jiffies.saturating_sub(prev.jiffies) as f64;
                    let d_total = total_jiffies - prev.total_jiffies;
                    if d_total > 0.0 {
                        let num_cores = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1) as f64;
                        (d_proc / d_total) * 100.0 * num_cores
                    } else {
                        0.0
                    }
                } else {
                    0.0
                };
                prev_map.insert(pid, CpuPrev { jiffies: proc_jiffies, total_jiffies });
                return pct;
            }
        }
    }
    0.0
}"""
content = content.replace(old_cpu, new_cpu)

# Add cleanup
old_cleanup = """    if !search.is_empty() {
        let lower = search.to_lowercase();
        procs.retain(|p| p.name.to_lowercase().contains(&lower));
    }"""

new_cleanup = """    {
        let mut prev_io = PREV_IO.lock().unwrap();
        prev_io.retain(|&pid, _| procs.iter().any(|p| p.pid == pid));
        let mut prev_cpu = PREV_CPU.lock().unwrap();
        prev_cpu.retain(|&pid, _| procs.iter().any(|p| p.pid == pid));
    }

    if !search.is_empty() {
        let lower = search.to_lowercase();
        procs.retain(|p| p.name.to_lowercase().contains(&lower));
    }"""
content = content.replace(old_cleanup, new_cleanup)

open("src/monitors/processes.rs", "w").write(content)
