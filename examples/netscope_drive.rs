//! Runtime validation for the NetScope extension.
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
        format!("{home}/.config/vanta/extensions/netscope.wasm")
    });
    let iters: u32 = std::env::args().nth(2).and_then(|s| s.parse().ok()).unwrap_or(6);

    println!("NetScope drive — loading: {path}");
    let _sampler = vanta::monitors::start(std::time::Duration::from_millis(500));
    std::thread::sleep(std::time::Duration::from_millis(1100));

    let manifest = Manifest::new([Wasm::file(&path)]).with_timeout(std::time::Duration::from_millis(10));
    let mut plugin = Plugin::new(&manifest, vanta::extension::host_api::functions(), true).expect("plugin");

    let render = |p: &mut Plugin, id: &str| -> (String, std::time::Duration) {
        let t = std::time::Instant::now();
        let out = match p.call::<&str, Vec<u8>>("render_widget", id) {
            Ok(b) => text(&b),
            Err(e) => format!("ERR {e}"),
        };
        (out, t.elapsed())
    };

    const WIDGETS: &[&str] = &["netscope", "netscope_table", "netscope_activity", "netscope_summary"];

    for i in 0..iters {
        println!("\n===== iteration {}/{iters}", i + 1);
        for &w in WIDGETS {
            let (out, dur) = render(&mut plugin, w);
            println!("--- {w}  ({dur:?})");
            if out.starts_with("ERR") {
                eprintln!("  ⚠ {w}: {out}");
            } else {
                for l in out.lines().filter(|l| !l.trim().is_empty()).take(12) {
                    println!("  {l}");
                }
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(1200));
    }

    println!("\n===== FINAL OUTPUT");
    for &w in WIDGETS {
        let (out, _) = render(&mut plugin, w);
        println!("\n--- {w}");
        for l in out.lines() { println!("  {l}"); }
    }

    let bench = 300u32;
    let t = std::time::Instant::now();
    for _ in 0..bench { let _ = render(&mut plugin, "netscope"); }
    let avg = t.elapsed() / bench;
    println!("\nper-render avg ({bench}x netscope): {avg:?}");

    let budget_ms: u128 = if cfg!(debug_assertions) { 50 } else { 10 };
    assert!(avg.as_millis() < budget_ms, "avg {avg:?} exceeds {budget_ms}ms budget");
    println!("\n✓ NetScope validated");
}
