use crossterm::event::KeyEvent;
use ratatui::layout::Rect;
use ratatui::Frame;
use toml::Value;

use crate::theme::Theme;

/// Metadata for a Vanta Extension.
#[derive(Debug, Clone)]
pub struct ExtensionMetadata {
    pub id: &'static str,
    pub name: &'static str,
    pub author: &'static str,
    pub version: &'static str,
    pub description: &'static str,
}

/// A Component (Widget) provided by an extension.
pub trait Component {
    fn id(&self) -> &'static str;
    fn render(&mut self, f: &mut Frame, area: Rect, theme: &Theme);
    fn handle_key(&mut self, _key: KeyEvent) -> bool {
        // Return true if the key was consumed
        false
    }
}

/// A full Page provided by an extension.
pub trait Page {
    fn id(&self) -> &'static str;
    fn title(&self) -> &'static str;
    fn render(&mut self, f: &mut Frame, area: Rect, theme: &Theme);
    fn handle_key(&mut self, _key: KeyEvent) -> bool {
        false
    }
    /// Called periodically (e.g., each frame or tick) for updates
    fn tick(&mut self) {}
}

/// The core Extension trait.
pub trait Extension {
    fn metadata(&self) -> ExtensionMetadata;

    /// Initialize the extension with its configuration namespace (if any).
    fn init(&mut self, _config: Option<&Value>) {}

    /// Return pages provided by this extension.
    fn pages(&self) -> Vec<Box<dyn Page>> {
        Vec::new()
    }

    /// Return individual components (widgets) provided by this extension.
    fn components(&self) -> Vec<Box<dyn Component>> {
        Vec::new()
    }

    /// Cleanup and gracefully shutdown background tasks.
    fn shutdown(&mut self) {}
}

/// Manager for handling registered extensions.
pub struct ExtensionManager {
    pub extensions: Vec<Box<dyn Extension>>,
}

impl ExtensionManager {
    pub fn new() -> Self {
        Self {
            extensions: Vec::new(),
        }
    }

    pub fn register(
        &mut self,
        mut ext: Box<dyn Extension>,
        global_ext_config: Option<&toml::Value>,
    ) {
        let meta = ext.metadata();

        // 1. Check if it's enabled in `[extensions.enabled]`
        let mut enabled = false;
        let mut ext_config = None;

        if let Some(cfg) = global_ext_config {
            // Read `enabled` array
            if let Some(enabled_arr) = cfg.get("enabled").and_then(|v| v.as_array()) {
                enabled = enabled_arr.iter().any(|v| v.as_str() == Some(meta.id));
            }
            // Extract `[extensions.<id>]` specific config
            ext_config = cfg.get(meta.id);
        }

        if enabled {
            ext.init(ext_config);
            self.extensions.push(ext);
        }
    }

    pub fn all_pages(&self) -> Vec<Box<dyn Page>> {
        let mut pages = Vec::new();
        for ext in &self.extensions {
            let mut ext_pages = ext.pages();
            pages.append(&mut ext_pages);
        }
        pages
    }

    pub fn shutdown_all(&mut self) {
        for ext in &mut self.extensions {
            ext.shutdown();
        }
    }
}
pub mod template;
