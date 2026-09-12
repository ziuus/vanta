use serde::{Deserialize, Serialize};

use crate::custom::config::CustomWidgetConfig;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub ui: UiConfig,
    pub widgets: WidgetConfig,
    /// User-defined custom widgets declared via `[[custom_widgets]]` entries.
    /// Existing configs that omit this field load fine — serde defaults to an
    /// empty `Vec`.
    pub custom_widgets: Vec<CustomWidgetConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct UiConfig {
    /// Seconds between data samples (CPU, memory, processes, ...).
    pub refresh_rate: f64,
    /// Render frames per second. Animations (visualizer, matrix, donut) run at this rate.
    pub fps: u32,
    pub theme: String,
    pub startup_mode: String,
    /// 24-hour clock (false = 12-hour with am/pm).
    pub clock_24h: bool,
    /// Visualizer style: bars | mirror | wave | peaks.
    pub visualizer: String,
    /// Gauge style: arc | bars | vertical.
    pub gauge_style: String,
    /// History graph style: block | braille.
    pub graph_style: String,
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            refresh_rate: 0.5,
            fps: 30,
            theme: "dark".to_string(),
            startup_mode: "dashboard".to_string(),
            clock_24h: true,
            visualizer: "bars".to_string(),
            gauge_style: "arc".to_string(),
            graph_style: "block".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct WidgetConfig {
    pub cpu: bool,
    pub memory: bool,
    pub disk: bool,
    pub network: bool,
    pub gpu: bool,
    pub clock: bool,
    pub calendar: bool,
    pub music_viz: bool,
    pub processes: bool,
    pub media: bool,
    pub matrix: bool,
    pub video: bool,
}

impl Default for WidgetConfig {
    fn default() -> Self {
        Self {
            cpu: true,
            memory: true,
            disk: true,
            network: true,
            gpu: true,
            clock: true,
            calendar: true,
            music_viz: true,
            processes: true,
            media: true,
            matrix: true,
            video: true,
        }
    }
}

impl Config {
    pub fn load() -> Self {
        let content = std::fs::read_to_string(config_path()).unwrap_or_default();
        let mut cfg: Config = toml::from_str(&content).unwrap_or_default();
        cfg.ui.refresh_rate = cfg.ui.refresh_rate.clamp(0.1, 10.0);
        cfg.ui.fps = cfg.ui.fps.clamp(5, 120);
        cfg
    }

    /// Persist only the runtime-mutable fields (`[ui]` and `[widgets]`).
    ///
    /// The `[[custom_widgets]]` array is entirely user-authored. Rewriting the
    /// whole file with `toml::to_string(self)` would serialise it as a flat
    /// inline array and destroy the user's hand-written table entries on every
    /// theme change or page switch. Instead we:
    ///   1. Parse the existing file into a `toml::Value` document tree.
    ///   2. Overwrite only the `[ui]` and `[widgets]` tables in that tree.
    ///   3. Write the mutated tree back — `[[custom_widgets]]` and any comments
    ///      survive untouched.
    pub fn save(&self) {
        let path = config_path();
        if let Some(parent) = std::path::Path::new(&path).parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        // Serialise only the two mutable sections.
        let Ok(ui_val) = toml::Value::try_from(&self.ui) else {
            return;
        };
        let Ok(widgets_val) = toml::Value::try_from(&self.widgets) else {
            return;
        };

        // Load the existing document so we can do a surgical update.
        let existing = std::fs::read_to_string(&path).unwrap_or_default();
        let mut doc: toml::Value =
            toml::from_str(&existing).unwrap_or(toml::Value::Table(toml::map::Map::new()));

        if let toml::Value::Table(ref mut map) = doc {
            map.insert("ui".to_string(), ui_val);
            map.insert("widgets".to_string(), widgets_val);
            // Deliberately do NOT touch "custom_widgets".
        }

        if let Ok(out) = toml::to_string(&doc) {
            let _ = std::fs::write(&path, out);
        }
    }
}

pub fn config_path() -> String {
    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        format!("{}/vanta/config.toml", xdg)
    } else if let Ok(home) = std::env::var("HOME") {
        format!("{}/.config/vanta/config.toml", home)
    } else {
        "config.toml".to_string()
    }
}
