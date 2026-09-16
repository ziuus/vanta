# Vanta Extension & Customization Guide

Vanta allows you to customize your terminal dashboard experience either without writing code (via `config.toml`) or by building reusable Rust extensions.

---

## 1. Zero-Code Customization (For Users)

You can customize your dashboard layout or create entirely new pages directly in `~/.config/vanta/config.toml`.

### A. Customizing the Main Dashboard Grid
Rearrange the layout by specifying columns and widget IDs in `[dashboard.layout]`:

```toml
[dashboard]
layout = [
    ["system", "cpu", "memory", "network"],
    ["clock", "processes"]
]
```

### B. Creating Custom Pages
Add `[[pages]]` to create dedicated tab views with custom layouts. They automatically appear in the top navigation bar with hotkeys (`1`, `2`, `3`, etc.):

```toml
[[pages]]
name = "DevOps View"
layout = [
    ["system", "network"],
    ["processes", "server_ping"]
]

[[pages]]
name = "Media Focus"
layout = [
    ["pinned_media", "video"],
    ["media", "clock"]
]
```

---

## 2. Building Rust Extensions (For Developers)

Extensions allow developers to build reusable widgets (`Component`) and full-screen views (`Page`) in Rust.

### A. Extension Traits Overview (`vanta::extension`)

1. **`Component`**: A single UI box (e.g. `server_ping`).
   ```rust
   pub trait Component: Send + Sync {
       fn id(&self) -> &'static str; // Unique identifier used in config layout
       fn render(&mut self, f: &mut Frame, area: Rect, theme: &Theme);
   }
   ```

2. **`Page`**: A full-screen view (e.g. `SecurityPage`).
   ```rust
   pub trait Page: Send + Sync {
       fn id(&self) -> &'static str;
       fn title(&self) -> &'static str; // Navigation title
       fn render(&mut self, f: &mut Frame, area: Rect, theme: &Theme);
   }
   ```

3. **`Extension`**: Bundles metadata, pages, and components together.
   ```rust
   pub trait Extension: Send + Sync {
       fn metadata(&self) -> ExtensionMetadata;
       fn init(&mut self, config: Option<&toml::Value>) {}
       fn pages(&self) -> Vec<Box<dyn Page>> { vec![] }
       fn components(&self) -> Vec<Box<dyn Component>> { vec![] }
       fn shutdown(&mut self) {}
   }
   ```

### B. Creating an Extension Crate
1. Create a new Rust library crate.
2. Add `vanta` as a dependency in your `Cargo.toml`:
   ```toml
   [dependencies]
   vanta = { git = "https://github.com/ziuus/vanta" }
   ratatui = "0.29"
   ```
3. Implement the `Extension` and `Component`/`Page` traits in `src/lib.rs`.

---

## 3. Registering & Using Extensions

### A. Register in Core (`src/main.rs`)
To include an extension in a Vanta build:
```rust
app.ext_manager.register(
    Box::new(my_extension_crate::MyExtension),
    app.config.extensions.as_ref()
);
```

### B. Enable in Config (`config.toml`)
Extensions are **disabled by default**. Users must explicitly enable them:
```toml
[extensions]
enabled = ["my_ext_id"]

[extensions.my_ext_id]
custom_setting = "value"
```

Once enabled, components exported by the extension (e.g. `server_ping`) can be placed directly into any layout grid in `config.toml`.
