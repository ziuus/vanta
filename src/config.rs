use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub ui: UiConfig,
    pub widgets: WidgetConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiConfig {
    pub refresh_rate: f64,
    pub theme: String,
    #[serde(default = "default_startup_mode")]
    pub startup_mode: String,
}

fn default_startup_mode() -> String {
    "overview".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
    pub cmatrix: bool,
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
            cmatrix: true,
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            ui: UiConfig {
                refresh_rate: 0.25,
                theme: "dark".to_string(),
                startup_mode: "overview".to_string(),
            },
            widgets: WidgetConfig::default(),
        }
    }
}

impl Config {
    pub fn load() -> Self {
        let config_path = dirs_or_default();
        let content = std::fs::read_to_string(&config_path).unwrap_or_default();
        toml::from_str(&content).unwrap_or_default()
    }

    pub fn save(&self) {
        let config_path = dirs_or_default();
        // Ensure parent directory exists before writing
        if let Some(parent) = std::path::Path::new(&config_path).parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(toml_str) = toml::to_string(self) {
            let _ = std::fs::write(&config_path, toml_str);
        }
    }
}

fn dirs_or_default() -> String {
    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        format!("{}/vanta/config.toml", xdg)
    } else if let Ok(home) = std::env::var("HOME") {
        format!("{}/.config/vanta/config.toml", home)
    } else {
        "config.toml".to_string()
    }
}
