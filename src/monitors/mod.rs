pub mod cpu;
pub mod disk;
pub mod gpu;
pub mod history;
pub mod memory;
pub mod network;
pub mod processes;
pub mod system_info;

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, LazyLock, Mutex};
use std::time::{Duration, Instant};

/// Headline numbers for the title bar and gauges. Rebuilt every sample.
#[derive(Clone, Debug, Default)]
pub struct Summary {
    pub cpu_pct: f32,
    pub mem_pct: f64,
    pub gpu_pct: Option<f64>,
    pub disk_pct: Option<f64>,
    pub rx_kbps: f64,
    pub tx_kbps: f64,
    pub battery: Option<(u8, bool)>,
    pub uptime: String,
    pub temp_c: Option<f64>,
}

static SUMMARY: LazyLock<Mutex<Summary>> = LazyLock::new(|| Mutex::new(Summary::default()));

pub fn summary() -> Summary {
    SUMMARY.lock().unwrap().clone()
}

fn collect_summary() -> Summary {
    let cpu = cpu::snapshot();
    let mem = memory::snapshot();
    let net = network::snapshot();
    Summary {
        cpu_pct: cpu.usage,
        mem_pct: mem.pct(),
        gpu_pct: gpu::util_pct(),
        disk_pct: disk::root_pct(),
        rx_kbps: net.rx_kbps,
        tx_kbps: net.tx_kbps,
        battery: system_info::read_battery(),
        uptime: system_info::fmt_uptime(sysinfo::System::uptime()),
        temp_c: cpu.max_temp(),
    }
}

/// Take one full sample of every monitor.
fn sample_all(sys: &mut sysinfo::System) {
    sys.refresh_cpu_all();
    sys.refresh_memory();
    cpu::sample(sys);
    memory::sample(sys);
    gpu::sample();
    network::sample();
    disk::sample();
    processes::sample(sys.total_memory());
    crate::widgets::media::sample();
    *SUMMARY.lock().unwrap() = collect_summary();
}

/// Start the background sampler. The returned handle holds the interval in
/// milliseconds; changing it takes effect on the next cycle.
pub fn start(interval: Duration) -> Arc<AtomicU64> {
    let handle = Arc::new(AtomicU64::new(interval.as_millis() as u64));
    let h = Arc::clone(&handle);
    std::thread::Builder::new()
        .name("vanta-sampler".into())
        .spawn(move || {
            let mut sys = sysinfo::System::new();
            // Two quick CPU refreshes so the first visible sample has real
            // usage numbers instead of zeros.
            sys.refresh_cpu_all();
            std::thread::sleep(sysinfo::MINIMUM_CPU_UPDATE_INTERVAL);
            loop {
                let started = Instant::now();
                sample_all(&mut sys);
                let interval = Duration::from_millis(h.load(Ordering::Relaxed).max(100));
                std::thread::sleep(interval.saturating_sub(started.elapsed()));
            }
        })
        .expect("spawn sampler thread");
    facts_thread();
    handle
}

/// `status::sample()` shells out to `checkupdates`, `nmcli`, `pacman` and
/// `docker` — measured at ~7s combined on a first run (checkupdates alone syncs
/// package DBs over the network). On the sampler thread that stalled every
/// monitor behind it: `SUMMARY` is assigned last, so the title bar and gauges
/// sat at their `Default` 0% with a blank uptime, and every graph held the
/// single data point from the one sample that had completed. Its cache is
/// already `Mutex`-guarded and TTL-gated, so it just needs a thread of its own.
fn facts_thread() {
    std::thread::Builder::new()
        .name("vanta-facts".into())
        .spawn(|| loop {
            crate::widgets::status::sample();
            // sample() early-returns on a lock check until its TTL expires, so
            // polling this often costs nothing.
            std::thread::sleep(Duration::from_secs(1));
        })
        .expect("spawn facts thread");
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Regression: the headline numbers must not be gated behind the slow
    /// shell-outs in `status::sample()`. With those inline on the sampler thread
    /// this took ~7s, so the first frames rendered `cpu 0%`, `mem 0%` and an
    /// empty uptime.
    #[test]
    fn summary_populates_before_the_slow_shell_outs_could_finish() {
        let _handle = start(Duration::from_millis(100));
        std::thread::sleep(Duration::from_secs(1));
        let s = summary();
        assert!(!s.uptime.is_empty(), "uptime still Default after 1s");
        assert!(s.mem_pct > 0.0, "mem_pct still Default after 1s");
    }
}
