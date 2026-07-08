use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub ui: UiConfig,
    pub widgets: WidgetConfig,
    #[serde(default)]
    pub demo: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiConfig {
    pub refresh_rate: f64,
    pub theme: String,
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
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            ui: UiConfig {
                refresh_rate: 0.5,
                theme: "dark".to_string(),
            },
            widgets: WidgetConfig::default(),
            demo: false,
        }
    }
}

impl Config {
    pub fn load() -> Self {
        let config_path = dirs_or_default();
        let content = std::fs::read_to_string(&config_path).unwrap_or_default();
        toml::from_str(&content).unwrap_or_default()
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
