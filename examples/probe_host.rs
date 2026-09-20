// Ad-hoc harness: loads a .wasm through the same path WasmExtension uses and
// prints the returned UI tree, so host capabilities can be probed headlessly.
use extism::{Manifest, Plugin, Wasm};

fn main() {
    let path = std::env::args()
        .nth(1)
        .expect("usage: probe_host <file.wasm>");
    let manifest =
        Manifest::new([Wasm::file(&path)]).with_timeout(std::time::Duration::from_millis(10));
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
    match plugin.call::<&str, Vec<u8>>("render_widget", "probe") {
        Ok(b) => println!("render:\n{}", String::from_utf8_lossy(&b)),
        Err(e) => println!("render ERR: {e}"),
    }
}
