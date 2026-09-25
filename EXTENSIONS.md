# Vanta Extension Developer Guide

Vanta is built to be highly extensible. Any developer can build custom widgets, data fetchers, and UI components using **WebAssembly (WASM)** and the **Extism** plugin system. 

This guide explains how to create your own Vanta plugin out of the box, compile it, and publish it to the community.

## 1. Extension Architecture

Vanta uses micro-extensions. Every individual widget (e.g., a Crypto tracker, a World Clock, a File Browser) is compiled as its own independent `.wasm` plugin.

Because the plugins run in an Extism sandbox:
- **They are secure:** They cannot randomly access your filesystem unless permitted by the host.
- **They can be written in many languages:** While we use Rust for first-party plugins, Extism supports Go, Zig, C, JS, and more.
- **They fetch data via the Host:** WASM plugins cannot perform native blocking I/O directly. They use `extism:host/env::http_request` for network requests, or query the Vanta host for system data via a Key-Value mailbox (`vanta_query`).

## 2. Setting Up a New Plugin (Rust)

Currently, the most supported way to build a Vanta plugin is using Rust.

1. Create a new Rust library crate:
   ```bash
   cargo new my_vanta_widget --lib
   cd my_vanta_widget
   ```

2. Update your `Cargo.toml` to include the Extism PDK (Plugin Development Kit) and Vanta's required types (if any):
   ```toml
   [package]
   name = "my_vanta_widget"
   version = "0.1.0"
   edition = "2021"

   [lib]
   crate-type = ["cdylib"]

   [dependencies]
   extism-pdk = "1.0"
   serde = { version = "1.0", features = ["derive"] }
   serde_json = "1.0"
   ```

## 3. Plugin Structure

Every Vanta widget needs to export a `render` function that Vanta's rendering engine will call every frame.

Here is a minimal example of a Vanta widget (`src/lib.rs`):

```rust
use extism_pdk::*;
use serde_json::json;

// This is called when the plugin is first loaded
#[plugin_fn]
pub fn init() -> FnResult<()> {
    // You can set up initial state, fetch APIs, etc.
    Ok(())
}

// This is called every frame to draw the widget
#[plugin_fn]
pub fn render() -> FnResult<String> {
    // Return a JSON representation of a Ratatui Paragraph/Block
    // Vanta's host engine will parse this JSON and render it in the terminal.
    
    let ui_json = json!({
        "type": "paragraph",
        "text": "Hello from my custom WASM widget!",
        "style": { "fg": "green" },
        "block": {
            "title": "My Widget",
            "borders": "ALL"
        }
    });

    Ok(ui_json.to_string())
}
```

## 4. Compiling to WebAssembly

To compile your plugin, you need the `wasm32-unknown-unknown` target installed.

```bash
# Install the WASM target (only needed once)
rustup target add wasm32-unknown-unknown

# Build the plugin in release mode
cargo build --target wasm32-unknown-unknown --release
```

This will produce a `.wasm` file at `target/wasm32-unknown-unknown/release/my_vanta_widget.wasm`.

## 5. Local Testing

You don't need to publish your plugin to test it. You can instantly link it to your local Vanta installation.

1. **Link the plugin:**
   Vanta has a built-in CLI command to symlink your compiled WASM file into the extensions folder (`~/.config/vanta/extensions/`).
   ```bash
   vanta link ./target/wasm32-unknown-unknown/release/my_vanta_widget.wasm
   ```

2. **Add it to your Vanta configuration:**
   Open your `~/.config/vanta/config.toml` (or run `vanta config`) and add your widget to a layout page:

   ```toml
   [extensions]
   # Ensure extensions are enabled
   enable = true

   [[pages]]
   name = "My Custom Page"
   layout = [
       ["my_vanta_widget", "system_info"],
       ["cpu_chart", "memory_chart"]
   ]
   ```

3. **Run Vanta:**
   Start Vanta, and you should see your custom widget running!

## 6. Publishing to the Community

Once your plugin is polished and ready for the world, you can publish it to the official Vanta community registry.

1. Fork the [vanta-integrations](https://github.com/vanta-ui/vanta-integrations) repository.
2. Copy your plugin's source code folder into the `components/` directory.
3. Add your crate to the workspace `Cargo.toml`.
4. Create a Pull Request against the main repository.

Once your PR is merged, the GitHub Actions CI pipeline will automatically compile your widget, calculate its SHA256 hash, and publish it to the official `registry.json`. 

Users worldwide will then be able to install your plugin just by adding it to their `config.toml`!
