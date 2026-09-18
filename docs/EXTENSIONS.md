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
