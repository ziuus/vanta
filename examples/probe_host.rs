// Ad-hoc harness: loads a .wasm through the same path WasmExtension uses and
// prints the returned UI tree, so host capabilities can be probed headlessly.
use extism::{Manifest, Plugin, Wasm};

fn main() {
    let path = std::env::args()
        .nth(1)
        .expect("usage: probe_host <file.wasm>");
    let manifest =
        Manifest::new([Wasm::file(&path)]).with_timeout(std::time::Duration::from_millis(250));
    // Start the real sampler so telemetry queries return live values, exactly
    // as they do inside the running app.
    let _sampler = vanta::monitors::start(std::time::Duration::from_millis(250));
    std::thread::sleep(std::time::Duration::from_millis(900));

    let mut plugin =
        Plugin::new(&manifest, vanta::extension::host_api::functions(), true).expect("plugin");
    let meta = plugin
        .call::<(), Vec<u8>>("metadata", ())
        .expect("metadata");
    println!("metadata: {}", String::from_utf8_lossy(&meta));
    let widgets = plugin
        .call::<(), Vec<u8>>("widgets", ())
        .map(|b| serde_json::from_slice::<Vec<String>>(&b).unwrap_or_default())
        .unwrap_or_default();
    println!("widgets: {widgets:?}");

    // Render each widget twice: the second pass proves cached state and
    // history survive across host calls.
    for pass in 1..=2 {
        for w in &widgets {
            let t = std::time::Instant::now();
            match plugin.call::<&str, Vec<u8>>("render_widget", w.as_str()) {
                Ok(b) => println!(
                    "\n--- {w} (pass {pass}, {:?}, {} bytes)\n{}",
                    t.elapsed(),
                    b.len(),
                    String::from_utf8_lossy(&b)
                ),
                Err(e) => println!("\n--- {w} ERR: {e}"),
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(1100));
    }
}
