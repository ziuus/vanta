# Vanta Extensions & Customization Guide

Vanta supports two ways to extend its UI:
1. **WASM Extensions (Recommended for Community Plugins):** No need to clone or rebuild Vanta. Build a standalone WebAssembly plugin, drop the `.wasm` file into `~/.config/vanta/extensions/`, and run `vanta`.
2. **Native Widgets (For Core Development):** Clone the repo and develop internal widgets directly in Rust with `cargo run`.

---

## 1. WASM Extensions (Zero-Rebuild Workflow)

WASM extensions run safely inside Vanta's sandboxed WebAssembly runtime (Extism / Wasmtime).

### How to Test / Develop a WASM Extension Locally

1. **Build the WASM binary:**
   Inside your plugin crate (e.g., `vanta-integrations/crypto_coin`):
   ```bash
   cargo build --target wasm32-wasip1 --release
   ```

2. **Place in the Vanta extensions directory:**
   Copy the output `.wasm` file into `~/.config/vanta/extensions/`:
   ```bash
   mkdir -p ~/.config/vanta/extensions/
   cp target/wasm32-wasip1/release/my_widget.wasm ~/.config/vanta/extensions/
   ```

3. **Enable it in `~/.config/vanta/config.toml`:**
   ```toml
   [extensions]
   enabled = ["my_widget"]

   [dashboard]
   layout = [
       ["system", "cpu", "memory"],
       ["my_widget", "processes"]
   ]
   ```

4. **Run Vanta:**
   ```bash
   vanta
   ```
   Vanta scans `~/.config/vanta/extensions/` at startup and mounts the extension. When you make code changes to your plugin, just re-run step 1 & 2 and restart Vanta.

---

## 2. Installing from Registry

Published community extensions can be installed directly with the CLI:

```bash
vanta ext search
vanta ext install crypto_coin
```

---

## 3. Native Widget Development (Core Contributors)

If you are developing a built-in monitor or widget for Vanta itself:

1. **Clone the repo:**
   ```bash
   git clone https://github.com/ziuus/vanta.git
   cd vanta
   ```
2. **Add your widget:**
   Create `src/widgets/my_widget.rs` and expose `render(f: &mut Frame, area: Rect, theme: &Theme)`.
3. **Register in `src/app.rs` or `src/screens/`:**
   Add your widget identifier to the layout dispatcher.
4. **Test immediately:**
   ```bash
   cargo run --bin vanta
   ```
   Or with release optimizations:
   ```bash
   cargo run --release
   ```

---

## 4. Host Telemetry API (`vanta_query`)

Extensions run sandboxed: **no filesystem, no network, no environment
variables, no host config**. A plugin that calls `std::fs::read_to_string
("/proc/stat")` gets `No such file or directory (os error 44)`. This is
deliberate — but it means a plugin cannot collect system data itself.

Instead the host exposes the telemetry it **already samples** through a single
host function:

```rust
use extism_pdk::*;

#[host_fn]
extern "ExtismHost" {
    fn vanta_query(request: String) -> String;
}

let json = unsafe { vanta_query(r#"{"topic":"cpu"}"#.to_string()) }?;
```

Request: `{"topic": "<name>"}` plus optional per-topic arguments
(`{"topic":"processes","limit":20}`). A bare string (`"cpu"`) also works.

Response: `{"ok":true,"data":{…}}` or `{"ok":false,"error":"…","topics":[…]}`.

### Topics

| Topic | Returns |
|---|---|
| `capabilities` | telemetry api version, host version, topic list, **and the list of things the host cannot provide** |
| `host` | host version, telemetry api version |
| `summary` | cpu/mem/gpu/disk %, rx/tx kbps, battery, uptime, temp |
| `cpu` | total %, per-core %, load 1/5/15, freq, per-sensor temps |
| `memory` | total/used/swap bytes and percentages |
| `network` | aggregate rx/tx rate and totals (`aggregate_only: true`) |
| `disk` | per-mount path, device, used/total bytes, used % |
| `gpu` | `present`, and when present name/util/temp/vram/clock |
| `processes` | `limit` (default 25, max 200) processes by CPU: pid, ppid, name, cmdline, cpu %, rss, state, threads, uid, io rates |

### Rules for extension authors

1. **Feature-detect.** Query `capabilities` and handle unknown topics; new
   hosts add topics over time.
2. **Never invent data.** If a topic is absent or returns `ok:false`, render an
   explicit unavailable state. `capabilities.unavailable` documents known gaps
   (per-interface network, connection tables, containers, git, journal) —
   these are genuinely not collected, so do not fake them.
3. **Compatibility.** A plugin that imports `vanta_query` will **fail to load
   on hosts older than 0.10.26** (unknown import). Plugins that need telemetry
   should declare `api_version = "0.9.2"` in their metadata and registry entry;
   plugins that do not import the host function keep working everywhere.
4. **Budget.** `render_widget` runs on the render thread with a **10 ms
   timeout**, every frame. Query only what you draw, and keep parsing cheap.

### Debugging

Load any plugin headlessly, with the real host functions and a live sampler:

```bash
cargo run --example probe_host -- /path/to/plugin.wasm    # debug profile; --release OOMs (LTO)
```
