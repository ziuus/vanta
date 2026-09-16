use extism::{Manifest, Plugin, Wasm};
use ratatui::layout::Rect;
use ratatui::Frame;
use serde_json;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use crate::extension::{Component, Extension, ExtensionMetadata};
use crate::protocol::{UiWidget};
use crate::theme::Theme;
use crate::ui_renderer;

pub struct WasmExtension {
    metadata: ExtensionMetadata,
    plugin: Arc<Mutex<Plugin>>,
    widgets: Vec<&'static str>,
}

impl WasmExtension {
    pub fn new(path: PathBuf) -> Result<Self, extism::Error> {
        let wasm = Wasm::file(&path);
        // Set a strict 10ms timeout on execution so WASM plugins cannot stall the Vanta render loop
        let manifest = Manifest::new([wasm]).with_timeout(std::time::Duration::from_millis(10));
        let mut plugin = Plugin::new(&manifest, [], true)?;

        // Call metadata function to get extension info
        let metadata_bytes = plugin.call::<(), Vec<u8>>("metadata", ())?;
        let metadata: ExtensionMetadata = serde_json::from_slice(&metadata_bytes)?;
        
        // Call widgets function to get list of widget IDs
        let widgets_bytes = plugin.call::<(), Vec<u8>>("widgets", ())?;
        let widgets: Vec<String> = serde_json::from_slice(&widgets_bytes)?;

        let leaked_widgets = widgets.into_iter().map(|w| Box::leak(w.into_boxed_str()) as &'static str).collect();

        Ok(Self {
            metadata,
            plugin: Arc::new(Mutex::new(plugin)),
            widgets: leaked_widgets,
        })
    }
}

impl Extension for WasmExtension {
    fn metadata(&self) -> ExtensionMetadata {
        self.metadata.clone()
    }

    fn components(&self) -> Vec<Box<dyn Component>> {
        self.widgets.iter().map(|id| {
            Box::new(WasmComponent {
                id: *id,
                plugin: Arc::clone(&self.plugin),
            }) as Box<dyn Component>
        }).collect()
    }
}

pub struct WasmComponent {
    id: &'static str,
    plugin: Arc<Mutex<Plugin>>,
}

impl Component for WasmComponent {
    fn id(&self) -> &'static str {
        self.id
    }

    fn render(&mut self, f: &mut Frame, area: Rect, _theme: &Theme) {
        let mut plugin = self.plugin.lock().unwrap();
        // Call the render_widget function on WASM side with the widget ID
        match plugin.call::<&str, Vec<u8>>("render_widget", self.id) {
            Ok(bytes) => {
                if let Ok(ui_widget) = serde_json::from_slice::<UiWidget>(&bytes) {
                    ui_renderer::render_widget(&ui_widget, f, area);
                } else {
                    // Render error text
                }
            }
            Err(_e) => {
                // Render error text
            }
        }
    }
}
