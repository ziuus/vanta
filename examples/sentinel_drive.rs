//! Drives a plugin through the real host loader while the machine is put
//! under load, so incident open/persist/recover can be observed end to end.
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
    let path = std::env::args().nth(1).unwrap();
    let load_secs: u64 = std::env::args()
        .nth(2)
        .and_then(|s| s.parse().ok())
        .unwrap_or(12);
    let _sampler = vanta::monitors::start(std::time::Duration::from_millis(400));
    std::thread::sleep(std::time::Duration::from_millis(900));

    let manifest =
        Manifest::new([Wasm::file(&path)]).with_timeout(std::time::Duration::from_millis(10));
    let mut plugin =
        Plugin::new(&manifest, vanta::extension::host_api::functions(), true).expect("plugin");

    // Saturate every core so the cpu/load thresholds are genuinely crossed.
    let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let mut handles = Vec::new();
    for _ in 0..std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
    {
        let s = stop.clone();
        handles.push(std::thread::spawn(move || {
            let mut x: u64 = 0;
            while !s.load(std::sync::atomic::Ordering::Relaxed) {
                x = x.wrapping_mul(6364136223846793005).wrapping_add(1);
                std::hint::black_box(x);
            }
        }));
    }

    let render = |p: &mut Plugin, id: &str| -> String {
        match p.call::<&str, Vec<u8>>("render_widget", id) {
            Ok(b) => text(&b),
            Err(e) => format!("ERR {e}"),
        }
    };

    let t0 = std::time::Instant::now();
    let mut phase = "LOAD";
    loop {
        let el = t0.elapsed().as_secs();
        if el >= load_secs && phase == "LOAD" {
            stop.store(true, std::sync::atomic::Ordering::Relaxed);
            phase = "RECOVER";
        }
        if el >= load_secs + 14 {
            break;
        }
        println!("\n===== t={el}s phase={phase}");
        print!("{}", render(&mut plugin, "sentinel"));
        if el >= load_secs {
            print!("{}", render(&mut plugin, "incident"));
        }
        std::thread::sleep(std::time::Duration::from_millis(2000));
    }

    for w in [
        "sentinel",
        "incident",
        "incidents",
        "sentinel_events",
        "incident_context",
        "sentinel_coverage",
    ] {
        println!("\n===== FINAL: {w}");
        print!("{}", render(&mut plugin, w));
    }

    let t = std::time::Instant::now();
    for _ in 0..200 {
        let _ = render(&mut plugin, "sentinel");
    }
    println!("\nper-render avg: {:?}", t.elapsed() / 200);
    for h in handles {
        let _ = h.join();
    }
}
