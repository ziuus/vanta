use extism::{Manifest, Plugin, Wasm};
use ratatui::layout::Rect;
use ratatui::Frame;
use serde_json;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use crate::extension::{Component, Extension, ExtensionMetadata};
use crate::protocol::UiWidget;
use crate::theme::Theme;
use crate::ui_renderer;

pub struct WasmExtension {
    metadata: ExtensionMetadata,
    plugin: Arc<Mutex<Plugin>>,
    widgets: Vec<String>,
    last_widgets: Arc<Mutex<HashMap<String, UiWidget>>>,
}

impl WasmExtension {
    pub fn new(path: PathBuf) -> Result<Self, extism::Error> {
        log::info!(target: "extension", "Loading WASM extension from {:?}", path);
        let wasm = Wasm::file(&path);
        // Set a reasonable 250ms timeout on execution so WASM plugins can perform I/O without stutter
        let manifest = Manifest::new([wasm])
            .with_allowed_hosts(
                vec![
                    "api.binance.com".to_string(),
                    "api.coingecko.com".to_string(),
                    "api.alternative.me".to_string(),
                ]
                .into_iter(),
            )
            .with_timeout(std::time::Duration::from_millis(250));
        // Host functions give the sandbox read-only access to telemetry the
        // sampler thread already collects; see extension::host_api.
        let mut plugin = Plugin::new(&manifest, crate::extension::host_api::functions(), true)?;

        // Call metadata function to get extension info
        let metadata_bytes = plugin.call::<(), Vec<u8>>("metadata", ())?;
        let metadata: ExtensionMetadata = serde_json::from_slice(&metadata_bytes)?;

        if !metadata.api_version.starts_with("0.9") {
            return Err(extism::Error::msg(format!(
                "Unsupported extension API version: {}",
                metadata.api_version
            )));
        }

        // Call widgets function to get list of widget IDs
        let widgets_bytes = plugin.call::<(), Vec<u8>>("widgets", ())?;
        let widgets: Vec<String> = serde_json::from_slice(&widgets_bytes)?;

        Ok(Self {
            metadata,
            plugin: Arc::new(Mutex::new(plugin)),
            widgets,
            last_widgets: Arc::new(Mutex::new(HashMap::new())),
        })
    }
}

impl Extension for WasmExtension {
    fn metadata(&self) -> ExtensionMetadata {
        self.metadata.clone()
    }

    fn components(&self) -> Vec<Box<dyn Component>> {
        self.widgets
            .iter()
            .map(|id| {
                Box::new(WasmComponent {
                    id: id.clone(),
                    plugin: Arc::clone(&self.plugin),
                    last_widgets: Arc::clone(&self.last_widgets),
                }) as Box<dyn Component>
            })
            .collect()
    }
}

pub struct WasmComponent {
    id: String,
    plugin: Arc<Mutex<Plugin>>,
    last_widgets: Arc<Mutex<HashMap<String, UiWidget>>>,
}

impl Component for WasmComponent {
    fn id(&self) -> &str {
        &self.id
    }

    fn render(&mut self, f: &mut Frame, area: Rect, _theme: &Theme) {
        // Extensions are opaque: give them a modest rate so live data and
        // simple animations update without paying for full fps WASM calls.
        crate::anim::request(10);
        let mut plugin = self.plugin.lock().unwrap();
        let mut rendered = false;
        // Call the render_widget function on WASM side with the widget ID
        match plugin.call::<&str, Vec<u8>>("render_widget", &self.id) {
            Ok(bytes) => {
                if bytes.len() <= crate::protocol::MAX_PAYLOAD_SIZE {
                    if let Ok(ui_widget) = serde_json::from_slice::<UiWidget>(&bytes) {
                        if ui_widget.validate().is_ok() {
                            ui_renderer::render_widget(&ui_widget, f, area);
                            self.last_widgets
                                .lock()
                                .unwrap()
                                .insert(self.id.clone(), ui_widget);
                            rendered = true;
                        }
                    }
                }
            }
            Err(_e) => {
                log::debug!(target: "extension", "render_widget error for {}: {:?}", self.id, _e);
            }
        }

        // Cache fallback: if this frame dropped/timed out, retain the previous valid frame to prevent blinking
        if !rendered {
            if let Some(prev) = self.last_widgets.lock().unwrap().get(&self.id) {
                ui_renderer::render_widget(prev, f, area);
            }
        }
    }

    fn handle_key(&mut self, key: crossterm::event::KeyEvent) -> bool {
        let mut plugin = self.plugin.lock().unwrap();
        if !plugin.function_exists("handle_key") {
            return false;
        }

        let key_str = match key.code {
            crossterm::event::KeyCode::Char(c) => c.to_string(),
            crossterm::event::KeyCode::Left => "left".to_string(),
            crossterm::event::KeyCode::Right => "right".to_string(),
            crossterm::event::KeyCode::Up => "up".to_string(),
            crossterm::event::KeyCode::Down => "down".to_string(),
            crossterm::event::KeyCode::Enter => "enter".to_string(),
            crossterm::event::KeyCode::Esc => "esc".to_string(),
            _ => return false,
        };

        let payload = serde_json::json!({
            "widget": self.id,
            "key": key_str
        });

        if let Ok(bytes) =
            plugin.call::<&str, Vec<u8>>("handle_key", &serde_json::to_string(&payload).unwrap())
        {
            if let Ok(handled) = serde_json::from_slice::<bool>(&bytes) {
                return handled;
            }
        }
        false
    }
}
