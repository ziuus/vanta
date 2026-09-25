pub mod agenda;
pub mod connections;
pub mod cpu;
pub mod crypto;
pub mod disk;
pub mod files;
pub mod fs_tasks;
pub mod gpu;
pub mod history;
pub mod memory;
pub mod network;
pub mod news;
pub mod obsidian;
pub mod processes;
pub mod services;
pub mod state_store;
pub mod system_info;
pub mod tasks;
pub mod weather;

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
    /// Hottest sensor, smoothed over ~10s so turbo spikes don't flicker.
    pub temp_c: Option<f64>,
    /// The CPU's critical temperature; alert levels are relative to it.
    pub temp_crit: f64,
    /// Latched "running hot" state (on at crit-10°, off below crit-15°).
    pub hot: bool,
}

impl Summary {
    /// Close enough to the throttle point to call critical.
    pub fn temp_critical(&self) -> bool {
        self.temp_c.is_some_and(|t| t >= self.temp_crit - 3.0)
    }
}

static SUMMARY: LazyLock<Mutex<Summary>> = LazyLock::new(|| Mutex::new(Summary::default()));

static STARTED: LazyLock<Instant> = LazyLock::new(Instant::now);
const WARMUP: Duration = Duration::from_secs(20);
const TEMP_TAU_SECS: f64 = 10.0;
const HOT_ON_BELOW_CRIT: f64 = 10.0;
const HOT_OFF_BELOW_CRIT: f64 = 15.0;

/// Time-based exponential moving average: `tau` is the time constant, so
/// the result doesn't depend on the sample interval.
fn ema(prev: Option<f64>, x: f64, dt: f64, tau: f64) -> f64 {
    match prev {
        Some(p) => p + (x - p) * (1.0 - (-dt / tau).exp()),
        None => x,
    }
}

fn hot_latch(was_hot: bool, temp: Option<f64>, crit: f64) -> bool {
    let margin = if was_hot {
        HOT_OFF_BELOW_CRIT
    } else {
        HOT_ON_BELOW_CRIT
    };
    temp.is_some_and(|t| t >= crit - margin)
}

static EPOCH: LazyLock<Instant> = LazyLock::new(Instant::now);

/// Tracks when a snapshot was last read, so samplers whose data only some
/// extensions consume can skip work nobody is looking at.
pub(crate) struct Demand {
    last_read_ms: AtomicU64,
    last_run_ms: AtomicU64,
}

impl Demand {
    pub(crate) const fn new() -> Self {
        Self {
            last_read_ms: AtomicU64::new(u64::MAX),
            last_run_ms: AtomicU64::new(0),
        }
    }

    fn now_ms() -> u64 {
        EPOCH.elapsed().as_millis() as u64
    }

    /// Record that a consumer just read the snapshot.
    pub(crate) fn touch(&self) {
        self.last_read_ms.store(Self::now_ms(), Ordering::Relaxed);
    }

    /// True if someone read the snapshot within the last 10s and at least
    /// `every` has passed since the previous run. Marks the run when true.
    pub(crate) fn due(&self, every: Duration) -> bool {
        let now = Self::now_ms();
        let read = self.last_read_ms.load(Ordering::Relaxed);
        if read == u64::MAX || now.saturating_sub(read) > 10_000 {
            return false;
        }
        let last = self.last_run_ms.load(Ordering::Relaxed);
        if last != 0 && now.saturating_sub(last) < every.as_millis() as u64 {
            return false;
        }
        self.last_run_ms.store(now.max(1), Ordering::Relaxed);
        true
    }
}

pub fn summary() -> Summary {
    SUMMARY.lock().unwrap().clone()
}

fn collect_summary(prev: &Summary, dt: f64) -> Summary {
    let cpu = cpu::snapshot();
    let mem = memory::snapshot();
    let net = network::snapshot();
    let temp_c = cpu
        .max_temp()
        .map(|t| ema(prev.temp_c, t, dt, TEMP_TAU_SECS));
    Summary {
        cpu_pct: cpu.usage,
        mem_pct: mem.pct(),
        gpu_pct: gpu::util_pct(),
        disk_pct: disk::root_pct(),
        rx_kbps: net.rx_kbps,
        tx_kbps: net.tx_kbps,
        battery: system_info::read_battery(),
        uptime: system_info::fmt_uptime(sysinfo::System::uptime()),
        // Vanta's own startup burst heats the CPU for a few seconds; don't
        // greet the user with an alert it caused.
        hot: STARTED.elapsed() > WARMUP && hot_latch(prev.hot, temp_c, *cpu::TEMP_CRIT),
        temp_crit: *cpu::TEMP_CRIT,
        temp_c,
    }
}

/// Take one full sample of every monitor.
///
/// Set `VANTA_PROFILE=1` to append per-step timings to /tmp/vanta-profile.log
/// — the only way to see where sampler time goes without a profiler.
fn sample_all(sys: &mut sysinfo::System) {
    let profiling = std::env::var_os("VANTA_PROFILE").is_some();
    let mut marks: Vec<(&str, u128)> = Vec::new();
    let mut t = Instant::now();
    let mut step = |name: &'static str, marks: &mut Vec<(&str, u128)>| {
        if profiling {
            marks.push((name, t.elapsed().as_micros()));
            t = Instant::now();
        }
    };

    sys.refresh_cpu_all();
    sys.refresh_memory();
    step("sysinfo", &mut marks);
    cpu::sample(sys);
    step("cpu", &mut marks);
    memory::sample(sys);
    gpu::sample();
    step("gpu", &mut marks);
    network::sample();
    step("net", &mut marks);
    services::sample();
    step("services", &mut marks);
    disk::sample();
    step("disk", &mut marks);
    processes::sample(sys.total_memory());
    step("procs", &mut marks);
    connections::sample();
    step("conns", &mut marks);
    crate::widgets::media::sample();
    step("media", &mut marks);
    static LAST: Mutex<Option<Instant>> = Mutex::new(None);
    let dt = LAST
        .lock()
        .unwrap()
        .replace(Instant::now())
        .map_or(0.0, |t| t.elapsed().as_secs_f64());
    let next = collect_summary(&SUMMARY.lock().unwrap(), dt);
    *SUMMARY.lock().unwrap() = next;
    step("summary", &mut marks);

    if profiling {
        use std::io::Write;
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .append(true)
            .create(true)
            .open("/tmp/vanta-profile.log")
        {
            let total: u128 = marks.iter().map(|m| m.1).sum();
            let line = marks
                .iter()
                .map(|(n, us)| format!("{}={:.1}ms", n, *us as f64 / 1000.0))
                .collect::<Vec<_>>()
                .join(" ");
            let _ = writeln!(f, "total={:.1}ms {}", total as f64 / 1000.0, line);
        }
    }
}

/// Start the background sampler. The returned handle holds the interval in
/// milliseconds; changing it takes effect on the next cycle.
pub fn start(interval: Duration) -> Arc<AtomicU64> {
    LazyLock::force(&STARTED);
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
    weather::start();
    tasks::start();
    crypto::start();
    agenda::start();
    let url = crate::config::Config::load().widgets.news_feed;
    news::start(url);
    let vault = crate::config::Config::load().ui.obsidian_vault;
    obsidian::start(vault);
    let mut home = std::path::PathBuf::from(".");
    if let Ok(h) = std::env::var("HOME") {
        home = std::path::PathBuf::from(h);
    }
    crate::monitors::files::init(&home);
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

    #[test]
    fn temp_smoothing_ignores_short_spikes() {
        let mut t = ema(None, 75.0, 0.0, TEMP_TAU_SECS);
        assert_eq!(t, 75.0);
        // A 2s turbo spike to 97° barely moves the smoothed value...
        for _ in 0..4 {
            t = ema(Some(t), 97.0, 0.5, TEMP_TAU_SECS);
        }
        assert!(t < 85.0, "spike leaked through: {t}");
        // ...but sustained heat gets there.
        for _ in 0..60 {
            t = ema(Some(t), 97.0, 0.5, TEMP_TAU_SECS);
        }
        assert!(t > 90.0);
    }

    #[test]
    fn hot_alert_is_relative_to_crit_with_hysteresis() {
        // Laptops that idle in the mid 80s (Tjmax 100) stay quiet.
        assert!(!hot_latch(false, Some(86.0), 100.0));
        assert!(hot_latch(false, Some(90.0), 100.0));
        assert!(hot_latch(true, Some(86.0), 100.0));
        assert!(!hot_latch(true, Some(84.0), 100.0));
        assert!(!hot_latch(true, None, 100.0));
        // A chip with a lower limit warns earlier.
        assert!(hot_latch(false, Some(86.0), 95.0));
    }

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
pub mod github;
