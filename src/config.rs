use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub ui: UiConfig,
    pub widgets: WidgetConfig,
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

    pub fn save(&self) {
        let path = config_path();
        if let Some(parent) = std::path::Path::new(&path).parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(toml_str) = toml::to_string(self) {
            let _ = std::fs::write(&path, toml_str);
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
