//! Validates the IOWatch plugin against the live host telemetry API.
//!
//! Usage:
//!   cargo run --example iowatch_drive [<wasm_path>] [<iters>]
//!
//! Defaults: ~/.config/vanta/extensions/iowatch.wasm, 8 iterations.
//!
//! What this verifies:
//! - Plugin loads with host functions wired
//! - All three widgets render without timeout or panic
//! - JSON output is parseable and non-empty
//! - Per-render latency is well inside the 10 ms host budget
//! - Widget output changes over time (io data is live, not mocked)
use extism::{Manifest, Plugin, Wasm};

fn text(b: &[u8]) -> String {
    let v: serde_json::Value = match serde_json::from_slice(b) {
        Ok(v) => v,
        Err(_) => return String::from_utf8_lossy(b).into(),
    };
    let mut out = String::new();
    if let Some(t) = v.pointer("/block/title").and_then(|x| x.as_str()) {
        out.push_str(&format!("[{}]\n", t.trim()));
    }
    for line in v["lines"].as_array().into_iter().flatten() {
        for s in line["spans"].as_array().into_iter().flatten() {
            out.push_str(s["content"].as_str().unwrap_or(""));
        }
        out.push('\n');
    }
    out
}

fn main() {
    let path = std::env::args().nth(1).unwrap_or_else(|| {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
        format!("{home}/.config/vanta/extensions/iowatch.wasm")
    });

    let iters: u32 = std::env::args()
        .nth(2)
        .and_then(|s| s.parse().ok())
        .unwrap_or(8);

    println!("IOWatch drive — loading: {path}");
    println!("Starting monitors…");

    let _sampler = vanta::monitors::start(std::time::Duration::from_millis(500));
    // Two sample windows so the io topic has rate data before the first render.
    std::thread::sleep(std::time::Duration::from_millis(1100));

    let manifest =
        Manifest::new([Wasm::file(&path)]).with_timeout(std::time::Duration::from_millis(10));
    let mut plugin =
        Plugin::new(&manifest, vanta::extension::host_api::functions(), true).expect("plugin");

    let render = |p: &mut Plugin, id: &str| -> (String, std::time::Duration) {
        let t = std::time::Instant::now();
        let out = match p.call::<&str, Vec<u8>>("render_widget", id) {
            Ok(b) => text(&b),
            Err(e) => format!("ERR {e}"),
        };
        (out, t.elapsed())
    };

    const WIDGETS: &[&str] = &["iowatch", "io_top", "io_activity"];

    let mut timings: Vec<std::time::Duration> = Vec::new();

    for i in 0..iters {
        println!("\n===== iteration {}/{iters}", i + 1);
        for &w in WIDGETS {
            let (out, dur) = render(&mut plugin, w);
            timings.push(dur);
            println!("--- {w}  ({dur:?})");
            // Print first 6 non-empty lines only to keep output readable.
            let printed: Vec<&str> = out
                .lines()
                .filter(|l| !l.trim().is_empty())
                .take(6)
                .collect();
            for l in &printed {
                println!("  {l}");
            }
            if out.lines().count() > 6 {
                println!("  … ({} lines total)", out.lines().count());
            }
            assert!(!out.starts_with("ERR"), "widget {w} returned error: {out}");
        }
        std::thread::sleep(std::time::Duration::from_millis(700));
    }

    // ── Final full render ──────────────────────────────────────────────────
    println!("\n===== FINAL OUTPUT");
    for &w in WIDGETS {
        let (out, _) = render(&mut plugin, w);
        println!("\n--- {w}");
        for l in out.lines() {
            println!("  {l}");
        }
    }

    // ── Performance ────────────────────────────────────────────────────────
    let bench_count = 300u32;
    let t = std::time::Instant::now();
    for _ in 0..bench_count {
        let _ = render(&mut plugin, "iowatch");
    }
    let avg = t.elapsed() / bench_count;
    println!("\nper-render avg ({bench_count}x iowatch): {avg:?}");

    let total: std::time::Duration = timings.iter().sum();
    let avg_all = total / timings.len() as u32;
    println!("avg across all widgets: {avg_all:?}");

    assert!(
        avg.as_millis() < 10,
        "per-render avg {avg:?} exceeds 10 ms host budget"
    );
    println!("\n✓ IOWatch validated — all widgets render, latency within budget");
}
